# mod-realmbox-presence calls public C++ APIs implemented by mod-playerbots.
# AzerothCore dynamic modules link only against game, not against each other,
# so both modules must be folded into the static modules archive.
ModuleNameToVariable("mod-playerbots" REALMBOX_PLAYERBOTS_LINKAGE_VAR)
ModuleNameToVariable("mod-realmbox-presence" REALMBOX_PRESENCE_LINKAGE_VAR)

if(NOT "${${REALMBOX_PRESENCE_LINKAGE_VAR}}" STREQUAL "disabled")
  if(NOT "${${REALMBOX_PRESENCE_LINKAGE_VAR}}" STREQUAL "static")
    message(FATAL_ERROR "mod-realmbox-presence must be built static")
  endif()
  if(NOT "${${REALMBOX_PLAYERBOTS_LINKAGE_VAR}}" STREQUAL "static")
    message(FATAL_ERROR "mod-realmbox-presence requires static mod-playerbots")
  endif()
endif()

if(BUILD_TESTING)
  set_property(GLOBAL APPEND PROPERTY ACORE_MODULE_TEST_SOURCES
    "${CMAKE_CURRENT_LIST_DIR}/tests/PresencePolicyTest.cpp")
  set_property(GLOBAL APPEND PROPERTY ACORE_MODULE_TEST_INCLUDES
    "${CMAKE_CURRENT_LIST_DIR}/src")
endif()
