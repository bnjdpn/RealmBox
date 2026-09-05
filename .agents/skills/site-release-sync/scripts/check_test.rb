#!/usr/bin/env ruby
# frozen_string_literal: true
require_relative "check"
require "tmpdir"

class SiteReleaseSyncTest
  SHA = "a" * 40
  FINGERPRINT = "c" * 64
  NOW = Time.utc(2026, 9, 5, 12)

  def assert_equal(expected, actual)
    raise "Expected #{expected.inspect}, got #{actual.inspect}" unless expected == actual
  end

  def assert_raises(type)
    begin
      yield
    rescue type
      return
    end
    raise "Expected #{type}"
  end

  def review
    { "source_commit" => SHA, "source_fingerprint" => FINGERPRINT, "version" => "2.0", "impact" => "none",
      "reason" => "Extract an internal helper with identical output and storage; public surfaces remain accurate.",
      "site" => "unchanged", "screenshots" => "unchanged", "catalogue" => "unchanged",
      "languages_reviewed" => [],
      "platforms" => [{ "platform" => "iOS", "availability" => "unchanged", "claims_changed" => false }] }
  end

  def validate(value)
    SiteReleaseSync.review!(value, commit: SHA, fingerprint: FINGERPRINT, now: NOW)
  end

  # Scenario 1: a real navigation change cannot ship with obsolete screenshots.
  def test_interface_change_needs_new_screenshots
    value = review.merge("impact" => "ui", "reason" => "Navigation and the session screen have changed.",
                         "site" => "updated", "languages_reviewed" => %w[fr-FR en-US])
    assert_raises(SiteReleaseSync::Invalid) { validate(value) }
    value["screenshots"] = "updated"
    result = validate(value)
    assert_equal "review_recorded", result["decision"]
    assert_equal false, result["publication_authorized"]
  end

  # Scenario 2: an internal refactor needs a reason, not manufactured web changes.
  def test_internal_refactor_requires_no_web_edits
    assert_equal "review_recorded", validate(review)["decision"]
    pending_refactor = review
    pending_refactor["platforms"][0]["availability"] = "pending"
    assert_equal "review_recorded", validate(pending_refactor)["decision"]
    assert_raises(SiteReleaseSync::Invalid) { validate(review.merge("reason" => "")) }
    assert_raises(SiteReleaseSync::Invalid) { validate(review.merge("site" => "updated")) }
  end

  # Scenario 3: pending features can be prepared without changing public promises.
  def test_pending_feature_preserves_public_availability
    value = review.merge("impact" => "ui", "site" => "deferred", "screenshots" => "deferred",
                         "catalogue" => "deferred", "languages_reviewed" => %w[fr-FR en-US],
                         "pending_work" => "Release candidate branch holds new pages and demo captures; await public store readback.")
    value["platforms"][0]["availability"] = "pending"
    result = validate(value)
    assert_equal "prepared_with_remaining_work", result["decision"]
    assert_equal false, result["remote_availability_verified_by_script"]
    value["platforms"][0]["claims_changed"] = true
    assert_raises(SiteReleaseSync::Invalid) { validate(value) }
  end

  def test_each_platform_needs_its_own_recent_public_readback
    value = review.merge("impact" => "content", "site" => "updated", "languages_reviewed" => %w[fr-FR en-US])
    value["platforms"][0].merge!("availability" => "released", "claims_changed" => true,
      "public_version" => "2.0", "public_url" => "https://apps.apple.com/app/id12345",
      "checked_at" => NOW.iso8601, "evidence" => "Public page lists 2.0 and its downloadable offer.")
    assert_equal "review_recorded", validate(value)["decision"]
    value["platforms"] << { "platform" => "Android", "availability" => "pending", "claims_changed" => true }
    assert_raises(SiteReleaseSync::Invalid) { validate(value) }
    value["platforms"].pop
    value["platforms"][0]["checked_at"] = (NOW - 8 * 86_400).iso8601
    assert_raises(SiteReleaseSync::Invalid) { validate(value) }
  end

  def test_review_cannot_be_reused_for_another_candidate
    assert_raises(SiteReleaseSync::Invalid) { validate(review.merge("source_commit" => "b" * 40)) }
  end

  def test_deferred_work_cannot_be_reported_as_complete
    value = review.merge("impact" => "content", "catalogue" => "deferred", "languages_reviewed" => %w[fr-FR en-US])
    assert_raises(SiteReleaseSync::Invalid) { validate(value) }
    value["pending_work"] = "Portfolio repository unavailable; change its platform/download entry after access is restored."
    assert_equal "prepared_with_remaining_work", validate(value)["decision"]
  end

  def test_same_commit_with_changed_public_sources_invalidates_review
    Dir.mktmpdir("site-sync-candidate-") do |directory|
      root = Pathname.new(directory)
      SiteReleaseSync.git!(root, "init", "-q")
      root.join("index.html").write("<h1>Public version</h1>")
      root.join(".gitignore").write("evidence/\n")
      SiteReleaseSync.git!(root, "add", ".")
      SiteReleaseSync.git!(root, "-c", "user.name=Site review test", "-c", "user.email=site-review@example.invalid", "commit", "-qm", "Fixture")
      before = SiteReleaseSync.identity(root)
      value = review.merge(before)
      root.join("index.html").write("<h1>Changed public promise</h1>")
      after = SiteReleaseSync.identity(root)
      assert_equal before["source_commit"], after["source_commit"]
      assert_raises(SiteReleaseSync::Invalid) do
        SiteReleaseSync.review!(value, commit: after["source_commit"], fingerprint: after["source_fingerprint"], now: NOW)
      end
      root.join("new-page.html").write("<h1>A new page</h1>")
      new_page = SiteReleaseSync.identity(root)
      raise "New public source omitted" if new_page == after
      root.join("evidence").mkdir
      root.join("evidence/review.json").write("{}")
      assert_equal new_page, SiteReleaseSync.identity(root)
    end
  end
end

tests = SiteReleaseSyncTest.new
scenarios = SiteReleaseSyncTest.instance_methods(false).grep(/^test_/).sort
scenarios.each do |name|
  tests.public_send(name)
  puts "PASS #{name}"
end
puts "#{scenarios.length} simulated scenarios passed; no publication or network calls."
