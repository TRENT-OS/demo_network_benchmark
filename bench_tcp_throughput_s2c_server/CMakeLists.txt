#
# Bench TCP Throughput
#
# Copyright (C) 2023, HENSOLDT Cyber GmbH
#

cmake_minimum_required(VERSION 3.17)

#-------------------------------------------------------------------------------
# use the SDK
find_package("os-sdk" REQUIRED)
os_sdk_set_defaults()
os_sdk_setup(CONFIG_FILE "system_config.h" CONFIG_PROJECT "system_config")

# Set additional include paths.
CAmkESAddCPPInclude("plat/${PLATFORM}")


#-------------------------------------------------------------------------------
project(bench_tcp_echo C)

# Overwrite the default log level of the lower layers to ERROR as the output
# otherwise gets too cluttered with debug prints.
set(LibUtilsDefaultZfLogLevel 5 CACHE STRING "" FORCE)

include("plat/${PLATFORM}/plat_nic.cmake")

DeclareCAmkESComponent(
    BenchTcpThroughput
    SOURCES
        components/BenchTcpThroughput/src/BenchTcpThroughput.c
    C_FLAGS
        -Wall -Werror
    LIBS
        system_config
        os_core_api
        lib_debug
        os_socket_client
)

NetworkStack_PicoTcp_DeclareCAmkESComponent(
    NwStack
    C_FLAGS
        -DNetworkStack_PicoTcp_USE_HARDCODED_IPADDR
        -DDEV_ADDR=ETH_ADDR
        -DGATEWAY_ADDR=ETH_GATEWAY_ADDR
        -DSUBNET_MASK=ETH_SUBNET_MASK
        -DOS_NETWORK_MAXIMUM_SOCKET_NO=NETWORK_STACK_NUM_SOCKETS
)

TimeServer_DeclareCAmkESComponent(
    TimeServer
)

os_sdk_create_CAmkES_system("BenchTcpThroughput.camkes")
