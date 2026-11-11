cmake_minimum_required(VERSION 3.21)
project(meganugget-desktop-qt VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

add_executable(meganugget-desktop-qt
  src/main.cpp
  src/MainWindow.cpp
  src/MainWindow.h
  src/ApplyChannelCli.cpp
  src/ApplyChannelCli.h
  src/DeviceScanner.cpp
  src/DeviceScanner.h
  src/Compatibility.cpp
  src/Compatibility.h
  src/Journal.cpp
  src/Journal.h
  src/CliRunner.cpp
  src/CliRunner.h
  src/PlistReader.cpp
  src/PlistReader.h
  src/IpaInstaller.cpp
  src/IpaInstaller.h
  src/TweakCatalog.cpp
  src/TweakCatalog.h
  src/TweakCompiler.cpp
  src/TweakCompiler.h
  src/ApplyChannel.cpp
  src/ApplyChannel.h
  resources/resources.qrc
)

target_include_directories(meganugget-desktop-qt PRIVATE src)
target_link_libraries(meganugget-desktop-qt PRIVATE Qt6::Widgets)

if(WIN32)
  set_target_properties(meganugget-desktop-qt PROPERTIES WIN32_EXECUTABLE TRUE)
endif()

install(TARGETS meganugget-desktop-qt RUNTIME DESTINATION bin)

option(MEGANUGGET_QT_TESTS "Build Qt unit tests" ON)
if(MEGANUGGET_QT_TESTS)
  enable_testing()
  find_package(Qt6 REQUIRED COMPONENTS Test)
  add_executable(apply-channel-cli-test
    tests/ApplyChannelCli_test.cpp
    src/ApplyChannelCli.cpp
    src/ApplyChannelCli.h
  )
  target_include_directories(apply-channel-cli-test PRIVATE src)
  target_link_libraries(apply-channel-cli-test PRIVATE Qt6::Test)
  add_test(NAME apply-channel-cli-test COMMAND apply-channel-cli-test)
endif()
