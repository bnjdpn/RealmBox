#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "optparse"
require "open3"
require "pathname"
require "time"
require "uri"
require "digest"
require "rbconfig"

module SiteReleaseSync
  class Invalid < StandardError; end
  ACTIONS = %w[unchanged updated deferred].freeze

  def self.require_text!(value, name)
    raise Invalid, "Missing #{name}" unless value.is_a?(String) && !value.strip.empty?
  end

  # This validates a reviewer decision, not the product or a remote store.
  def self.review!(review, commit: nil, fingerprint: nil, now: Time.now)
    raise Invalid, "Review must be a JSON object" unless review.is_a?(Hash)
    %w[source_commit source_fingerprint version reason].each { |key| require_text!(review[key], key) }
    raise Invalid, "source_commit must be a full Git SHA" unless review["source_commit"].match?(/\A[0-9a-f]{40,64}\z/)
    raise Invalid, "source_fingerprint must be SHA-256" unless review["source_fingerprint"].match?(/\A[0-9a-f]{64}\z/)
    raise Invalid, "Review belongs to another commit; review this candidate again" if commit && review["source_commit"] != commit
    raise Invalid, "Candidate files changed after the review; review the current sources again" if fingerprint && review["source_fingerprint"] != fingerprint
    raise Invalid, "impact must be none, ui or content" unless %w[none ui content].include?(review["impact"])
    %w[site screenshots catalogue].each do |key|
      raise Invalid, "Invalid #{key} decision" unless ACTIONS.include?(review[key])
    end
    locales = review["languages_reviewed"]
    raise Invalid, "languages_reviewed must list reviewed locales" unless locales.is_a?(Array) && locales.all? { |v| v.is_a?(String) && !v.strip.empty? }
    raise Invalid, "Public changes require a language review" if review["impact"] != "none" && locales.empty?
    if review["impact"] == "none" && %w[site screenshots catalogue].any? { |key| review[key] != "unchanged" }
      raise Invalid, "No public impact must not require artificial web edits"
    end
    if review["impact"] == "ui" && review["screenshots"] == "unchanged"
      raise Invalid, "UI impact requires updated captures or explicit deferral"
    end
    if review["screenshots"] == "deferred" && review["site"] == "updated"
      raise Invalid, "Do not publish a changed UI presentation with deferred captures"
    end
    deferred = %w[site screenshots catalogue].any? { |key| review[key] == "deferred" }
    require_text!(review["pending_work"], "pending_work for deferred work") if deferred
    platforms = review["platforms"]
    raise Invalid, "List each affected platform separately" unless platforms.is_a?(Array) && !platforms.empty?
    names = platforms.map { |platform| platform.is_a?(Hash) ? platform["platform"] : nil }
    raise Invalid, "Duplicate platform" unless names.uniq.length == names.length
    platforms.each do |platform|
      raise Invalid, "Platform must be an object" unless platform.is_a?(Hash)
      require_text!(platform["platform"], "platform")
      raise Invalid, "Invalid availability" unless %w[unchanged pending released].include?(platform["availability"])
      raise Invalid, "claims_changed must be boolean" unless [true, false].include?(platform["claims_changed"])
      next unless platform["claims_changed"]

      raise Invalid, "Pending/unknown releases cannot become public download promises" unless platform["availability"] == "released"
      %w[public_version public_url checked_at evidence].each { |key| require_text!(platform[key], key) }
      url = URI.parse(platform["public_url"])
      raise Invalid, "Public evidence requires an HTTPS destination" unless url.scheme == "https" && url.host && !url.userinfo
      checked_at = Time.iso8601(platform["checked_at"])
      raise Invalid, "Public readback is stale or in the future" unless (now - 7 * 86_400..now + 300).cover?(checked_at)
    end
    if review["impact"] == "none" && platforms.any? { |platform| platform["claims_changed"] }
      raise Invalid, "A changed availability claim is public impact"
    end
    pending = review["impact"] != "none" && platforms.any? { |platform| platform["availability"] == "pending" }
    { "decision" => (deferred || pending ? "prepared_with_remaining_work" : "review_recorded"),
      "pending_work" => review["pending_work"], "remote_availability_verified_by_script" => false,
      "publication_authorized" => false }
  rescue URI::InvalidURIError, ArgumentError => error
    raise Invalid, error.message
  end

  def self.git!(root, *arguments)
    output, status = Open3.capture2("git", "-C", root.to_s, *arguments)
    raise Invalid, "Cannot inspect candidate sources" unless status.success?
    output
  end

  # HEAD identifies unchanged tracked content; hash changed and non-ignored new
  # files too, so an uncommitted edit cannot reuse an earlier review. Ignored
  # build/evidence directories remain under the existing artifact contracts.
  def self.identity(root)
    sha = git!(root, "rev-parse", "HEAD").strip
    changed = git!(root, "diff", "--name-only", "--no-renames", "-z", "HEAD").split("\0")
    untracked = git!(root, "ls-files", "--others", "--exclude-standard", "-z").split("\0")
    digest = Digest::SHA256.new.update("site-release-sync-v1\0#{sha}\0")
    (changed + untracked).uniq.sort.each do |name|
      path = root.join(name)
      digest.update("#{name.bytesize}:#{name}\0")
      if path.symlink?
        digest.update("symlink\0#{path.readlink}\0")
      elsif path.file?
        digest.update("file\0#{path.stat.mode & 0o777}\0#{Digest::SHA256.file(path).hexdigest}\0")
      elsif path.directory?
        # A submodule update is represented by its own Git commit and status.
        digest.update("submodule\0#{git!(path, 'rev-parse', 'HEAD')}#{git!(path, 'status', '--porcelain')}\0")
      else
        digest.update("deleted\0")
      end
    end
    { "source_commit" => sha, "source_fingerprint" => digest.hexdigest }
  end

  def self.check_repository!(root)
    skill = root.join(".agents/skills/site-release-sync")
    %w[SKILL.md references/site-map.md references/review.md scripts/check_test.rb].each do |name|
      raise Invalid, "Missing portable workflow file: #{name}" unless skill.join(name).file?
    end
    agents = root.join("AGENTS.md")
    raise Invalid, "AGENTS.md must reference site-release-sync" unless agents.file? && agents.read.include?(".agents/skills/site-release-sync/SKILL.md")
    map = skill.join("references/site-map.md").read
    map.scan(/\]\(([^)]+)\)/).flatten.each do |target|
      next if target.match?(/\A(?:https?:|#)/)
      path = skill.join("references", target.split("#", 2).first).cleanpath
      raise Invalid, "Broken site map source: #{target}" unless path.exist?
    end
    config_path = root.join("marketing/site.json")
    if config_path.file?
      config = JSON.parse(config_path.read)
      required = config.fetch("locales").map { |entry| entry.fetch("code") }
      required.each do |locale|
        raise Invalid, "Missing locale route #{locale}" unless root.join(config.fetch("docs_dir"), locale, "index.html").file?
      end
      config.fetch("local_assets", []).each do |asset|
        raise Invalid, "Missing source asset #{asset['source']}" unless root.join(asset.fetch("source")).file?
      end
      tests = root.join("scripts/marketing_site_test.rb")
      raise Invalid, "Missing public snapshot regression tests: #{tests}" unless tests.file?
      output, status = Open3.capture2e(RbConfig.ruby, tests.to_s, chdir: root.to_s)
      puts output
      raise Invalid, "Public snapshot regression tests failed" unless status.success?
    end
    puts "Portable site workflow and local references checked. Run the site's generation/link checks from site-map.md."
    puts "Content, captures, legal meaning and public availability still require review. No publication performed."
  end

  def self.run(argv)
    options = { review: ENV["SITE_REVIEW_FILE"] }
    parser = OptionParser.new do |opts|
      opts.banner = "Usage: ruby .agents/skills/site-release-sync/scripts/check.rb [--check] [--review FILE] [--require-review]"
      opts.on("--check", "Check workflow, mapped sources, locale assets and public snapshot regression tests") { options[:check] = true }
      opts.on("--review FILE", "Validate reviewer decision from existing release artifacts") { |value| options[:review] = value }
      opts.on("--require-review", "Fail if the release review is absent") { options[:required] = true }
      opts.on("--fingerprint", "Print the commit and current source fingerprint for a review") { options[:fingerprint] = true }
      opts.on("-h", "--help") { puts opts; return 0 }
    end
    parser.parse!(argv)
    raise Invalid, "Unexpected arguments: #{argv.join(' ')}" unless argv.empty?
    root = Pathname.new(__dir__).join("../../../..").realpath
    check_repository!(root) if options[:check]
    puts JSON.pretty_generate(identity(root)) if options[:fingerprint]
    if options[:review] && !options[:review].empty?
      review_path = Pathname.new(options[:review]).realpath
      if review_path.to_s.start_with?("#{root}/")
        relative = review_path.relative_path_from(root).to_s
        _output, ignored = Open3.capture2("git", "-C", root.to_s, "check-ignore", "-q", "--", relative)
        raise Invalid, "Put the review outside the source tree or in its ignored release artifacts" unless ignored.success?
      end
      source = identity(root)
      result = review!(JSON.parse(review_path.read), commit: source.fetch("source_commit"), fingerprint: source.fetch("source_fingerprint"))
      puts JSON.pretty_generate(result)
    elsif options[:required]
      raise Invalid, "Set SITE_REVIEW_FILE to this candidate's review; see .agents/skills/site-release-sync/references/review.md"
    elsif !options[:check] && !options[:fingerprint]
      puts parser
    end
    0
  rescue Invalid, JSON::ParserError, KeyError, Errno::ENOENT, OptionParser::ParseError => error
    warn "Site release review: #{error.message}"
    1
  end
end

exit SiteReleaseSync.run(ARGV) if $PROGRAM_NAME == __FILE__
