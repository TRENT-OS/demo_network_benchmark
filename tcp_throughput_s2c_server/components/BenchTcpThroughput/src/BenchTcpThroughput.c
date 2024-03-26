/*
 * Bench TCP Throughput
 *
 * Copyright (C) 2023-2024, HENSOLDT Cyber GmbH
 * 
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * For commercial licensing, contact: info.cyber@hensoldt.net
 */

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
waitForIncomingConnection(
    const int srvHandleId)
{
    OS_Error_t ret;

    // Wait for the event letting us know that the connection was successfully
    // established.
    for (;;)
    {
        ret = OS_Socket_wait(&networkStackCtx);
        if (ret != OS_SUCCESS)
        {
            Debug_LOG_ERROR("OS_Socket_wait() failed, code %d", ret);
            break;
        }

        char evtBuffer[128];
        const size_t evtBufferSize = sizeof(evtBuffer);
        int numberOfSocketsWithEvents;

        ret = OS_Socket_getPendingEvents(
                  &networkStackCtx,
                  evtBuffer,
                  evtBufferSize,
                  &numberOfSocketsWithEvents);
        if (ret != OS_SUCCESS)
        {
            Debug_LOG_ERROR("OS_Socket_getPendingEvents() failed, code %d",
                            ret);
            break;
        }

        if (numberOfSocketsWithEvents == 0)
        {
            Debug_LOG_TRACE("OS_Socket_getPendingEvents() returned "
                            "without any pending events");
            continue;
        }

        // We only opened one socket, so if we get more events, this is not ok.
        if (numberOfSocketsWithEvents != 1)
        {
            Debug_LOG_ERROR("OS_Socket_getPendingEvents() returned with "
                            "unexpected #events: %d", numberOfSocketsWithEvents);
            ret = OS_ERROR_INVALID_STATE;
            break;
        }

        OS_Socket_Evt_t event;
        memcpy(&event, evtBuffer, sizeof(event));

        if (event.socketHandle != srvHandleId)
        {
            Debug_LOG_ERROR("Unexpected handle received: %d, expected: %d",
                            event.socketHandle, srvHandleId);
            ret = OS_ERROR_INVALID_HANDLE;
            break;
        }

        // Socket has been closed by NetworkStack component.
        if (event.eventMask & OS_SOCK_EV_FIN)
        {
            Debug_LOG_ERROR("OS_Socket_getPendingEvents() returned "
                            "OS_SOCK_EV_FIN for handle: %d",
                            event.socketHandle);
            ret = OS_ERROR_NETWORK_CONN_REFUSED;
            break;
        }

        // Incoming connection received.
        if (event.eventMask & OS_SOCK_EV_CONN_ACPT)
        {
            Debug_LOG_DEBUG("OS_Socket_getPendingEvents() returned "
                            "connection established for handle: %d",
                            event.socketHandle);
            ret = OS_SUCCESS;
            break;
        }

        // Remote socket requested to be closed only valid for clients.
        if (event.eventMask & OS_SOCK_EV_CLOSE)
        {
            Debug_LOG_ERROR("OS_Socket_getPendingEvents() returned "
                            "OS_SOCK_EV_CLOSE for handle: %d",
                            event.socketHandle);
            ret = OS_ERROR_CONNECTION_CLOSED;
            break;
        }

        // Error received - print error.
        if (event.eventMask & OS_SOCK_EV_ERROR)
        {
            Debug_LOG_ERROR("OS_Socket_getPendingEvents() returned "
                            "OS_SOCK_EV_ERROR for handle: %d, code: %d",
                            event.socketHandle, event.currentError);
            ret = event.currentError;
            break;
        }
    }

    return ret;
}

static OS_Error_t
sendTraffic(OS_Socket_Handle_t hSocket) {
    static uint8_t buf[OS_DATAPORT_DEFAULT_SIZE];

    // Initialize buffer.
    for (int i = 0; i < sizeof(buf); ++i) {
        buf[i] = i;
    }
    
    OS_Error_t ret;

    do
    {
        size_t actualLen = 0;

        ret = OS_Socket_write(
                  hSocket,
                  buf,
                  sizeof(buf),
                  &actualLen);

        // Make sure the compiler does not optimize the buffer away.
        __asm__ volatile("" : "+g" (buf) : :);

        switch (ret)
        {
        case OS_SUCCESS:
            break;

        case OS_ERROR_TRY_AGAIN:
            Debug_LOG_TRACE(
                "OS_Socket_read() reported try again");

            // Donate the remaining timeslice to a thread of the same
            // priority and try to read again with the next turn.
            seL4_Yield();
            break;

        case OS_ERROR_CONNECTION_CLOSED:
        case OS_ERROR_NETWORK_CONN_SHUTDOWN:
            Debug_LOG_INFO(
                "OS_Socket_read() reported connection closed");
            break;

        default:
            Debug_LOG_ERROR(
                "OS_Socket_read() failed, error %d", ret);
            return ret;
        }
    }
    while (ret == OS_SUCCESS || ret == OS_ERROR_TRY_AGAIN);

    return ret;
}

int
run(void)
{
    Debug_LOG_INFO("Starting Bench TCP Throughput S2C");

    // Check and wait until the NetworkStack component is up and running.
    OS_Error_t ret = waitForNetworkStackInit(&networkStackCtx);
    if (OS_SUCCESS != ret)
    {
        Debug_LOG_ERROR("waitForNetworkStackInit() failed with: %d", ret);
        return -1;
    }

    OS_Socket_Handle_t hServer;
    ret = OS_Socket_create(
              &networkStackCtx,
              &hServer,
              OS_AF_INET,
              OS_SOCK_STREAM);
    if (ret != OS_SUCCESS)
    {
        Debug_LOG_ERROR("OS_Socket_create() failed, code %d", ret);
        return -1;
    }

    const OS_Socket_Addr_t dstAddr =
    {
        .addr = OS_INADDR_ANY_STR,
        .port = BENCH_TCP_THROUGHPUT_PORT,
    };

    ret = OS_Socket_bind(
              hServer,
              &dstAddr);
    if (ret != OS_SUCCESS)
    {
        Debug_LOG_ERROR("OS_Socket_bind() failed, code %d", ret);
        OS_Socket_close(hServer);
        return -1;
    }

    ret = OS_Socket_listen(
              hServer,
              1);
    if (ret != OS_SUCCESS)
    {
        Debug_LOG_ERROR("OS_Socket_listen() failed, code %d", ret);
        OS_Socket_close(hServer);
        return -1;
    }

    for (;;)
    {
        Debug_LOG_INFO("Accepting new connection");
        OS_Socket_Handle_t hSocket;
        OS_Socket_Addr_t srcAddr = {0};

        do
        {
            ret = waitForIncomingConnection(hServer.handleID);
            if (ret != OS_SUCCESS)
            {
                Debug_LOG_ERROR("waitForIncomingConnection() failed, error %d", ret);
                OS_Socket_close(hSocket);
                return -1;
            }

            ret = OS_Socket_accept(
                      hServer,
                      &hSocket,
                      &srcAddr);
        }
        while (ret == OS_ERROR_TRY_AGAIN);
        if (ret != OS_SUCCESS)
        {
            Debug_LOG_ERROR("OS_Socket_accept() failed, error %d", ret);
            OS_Socket_close(hSocket);
            return -1;
        }

        ret = sendTraffic(hSocket);
        if (ret == OS_ERROR_CONNECTION_CLOSED || ret == OS_ERROR_NETWORK_CONN_SHUTDOWN) {
            Debug_LOG_INFO("echoTraffic() reported connection closed");
        } else if (ret != OS_SUCCESS) {
            Debug_LOG_ERROR("echoTraffic() failed, error %d", ret);
        }

        OS_Socket_close(hSocket);
    }

    return 0;
}
