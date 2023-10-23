/*
 * Bench UDP Throughput
 *
 * Copyright (C) 2023, HENSOLDT Cyber GmbH
 */

#include "network/OS_SocketTypes.h"
#include "system_config.h"

#include "lib_debug/Debug.h"
#include <camkes.h>
#include <string.h>

#include "OS_Error.h"
#include "OS_Socket.h"

static const if_OS_Socket_t networkStackCtx =
    IF_OS_SOCKET_ASSIGN(networkStack);

static OS_Error_t
waitForNetworkStackInit(
    const if_OS_Socket_t* const ctx)
{
    OS_NetworkStack_State_t networkStackState;

    for (;;)
    {
        networkStackState = OS_Socket_getStatus(ctx);
        if (networkStackState == RUNNING)
        {
            // NetworkStack up and running.
            return OS_SUCCESS;
        }
        else if (networkStackState == FATAL_ERROR)
        {
            // NetworkStack will not come up.
            Debug_LOG_ERROR("A FATAL_ERROR occurred in the Network Stack component.");
            return OS_ERROR_ABORTED;
        }

        // Yield to wait until the stack is up and running.
        seL4_Yield();
    }
}

static OS_Error_t
handleTraffic(OS_Socket_Handle_t hSocket) {
    static uint8_t buf[OS_DATAPORT_DEFAULT_SIZE];
    OS_Error_t ret;
    uint64_t totalReceived = 0;

    do
    {
        size_t actualLen = 0;

        OS_Socket_Addr_t srcAddr;
        ret = OS_Socket_recvfrom(
                  hSocket,
                  buf,
                  sizeof(buf),
                  &actualLen,
                  &srcAddr);

        // Make sure the compiler does not optimize the buffer away.
        __asm__ volatile("" : "+g" (buf) : :);

        switch (ret)
        {
        case OS_SUCCESS:
            if (actualLen == 0) {
                break;
            }
            if (buf[0] == 0) {
                totalReceived += actualLen;
            }
            if (buf[0] == 2) {
                totalReceived = 0;
            }
            if (buf[0] == 1 || buf[0] == 2) {
                buf[0] = totalReceived;
                buf[1] = totalReceived >> 8;
                buf[2] = totalReceived >> 16;
                buf[3] = totalReceived >> 24;
                buf[4] = totalReceived >> 32;
                buf[5] = totalReceived >> 40;
                buf[6] = totalReceived >> 48;
                buf[7] = totalReceived >> 56;
                do {
                    ret = OS_Socket_sendto(
                              hSocket,
                              buf,
                              8,
                              &actualLen,
                              &srcAddr);
                    if (ret == OS_SUCCESS && actualLen == 8) {
                        break;
                    }
                    if (ret != OS_ERROR_TRY_AGAIN) {
                        Debug_LOG_ERROR("OS_Socket_sendto() failed, error %d", ret);
                        return ret;
                    }
                } while (ret == OS_ERROR_TRY_AGAIN);
            }
            break;

        case OS_ERROR_TRY_AGAIN:
            Debug_LOG_TRACE(
                "OS_Socket_recvfrom() reported try again");

            // Donate the remaining timeslice to a thread of the same
            // priority and try to read again with the next turn.
            seL4_Yield();
            break;

        default:
            Debug_LOG_ERROR(
                "OS_Socket_recvfrom() failed, error %d", ret);
            return ret;
        }
    }
    while (ret == OS_SUCCESS || ret == OS_ERROR_TRY_AGAIN);

    return ret;
}

int
run(void)
{
    Debug_LOG_INFO("Starting Bench UDP Throughput");

    // Check and wait until the NetworkStack component is up and running.
    OS_Error_t ret = waitForNetworkStackInit(&networkStackCtx);
    if (OS_SUCCESS != ret)
    {
        Debug_LOG_ERROR("waitForNetworkStackInit() failed with: %d", ret);
        return -1;
    }

    OS_Socket_Handle_t hSocket;
    ret = OS_Socket_create(
              &networkStackCtx,
              &hSocket,
              OS_AF_INET,
              OS_SOCK_DGRAM);
    if (ret != OS_SUCCESS)
    {
        Debug_LOG_ERROR("OS_Socket_create() failed, code %d", ret);
        return -1;
    }

    const OS_Socket_Addr_t dstAddr =
    {
        .addr = OS_INADDR_ANY_STR,
        .port = BENCH_UDP_THROUGHPUT_PORT,
    };

    ret = OS_Socket_bind(
              hSocket,
              &dstAddr);
    if (ret != OS_SUCCESS)
    {
        Debug_LOG_ERROR("OS_Socket_bind() failed, code %d", ret);
        OS_Socket_close(hSocket);
        return -1;
    }

    ret = handleTraffic(hSocket);
    if (ret == OS_ERROR_CONNECTION_CLOSED || ret == OS_ERROR_NETWORK_CONN_SHUTDOWN) {
        Debug_LOG_INFO("echoTraffic() reported connection closed");
    } else if (ret != OS_SUCCESS) {
        Debug_LOG_ERROR("echoTraffic() failed, error %d", ret);
        OS_Socket_close(hSocket);
        return -1;
    }

    OS_Socket_close(hSocket);

    return 0;
}
