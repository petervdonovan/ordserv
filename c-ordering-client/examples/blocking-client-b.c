#include "c-ordering-client.h"

#include <stdio.h>

int main(int argc, char **argv) {
  OrderingClientApi* api = load_ordering_client_api();
  // let (client, _jh) = ordering_server::client::BlockingClient::start(("127.0.0.1", 15045), 1);
  ClientAndJoinHandle client_and_jh = api->start_client(1);
  void* client = client_and_jh.client;
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B99", 1, 0)));
  printf("B is starting maybe do\n");
  api->tracepoint_maybe_do(client, "B99", 1, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B0", 1, 0)));
  api->tracepoint_maybe_do(client, "B0", 1, 0);
  // println!("      words");
  printf("      words\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B0", 1, 0)));
  api->tracepoint_maybe_do(client, "B0", 1, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B0", 1, 1)));
  api->tracepoint_maybe_do(client, "B0", 1, 1);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B1", 1, 0)));
  api->tracepoint_maybe_do(client, "B1", 1, 0);
  // println!("      this");
  printf("      this\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B1", 1, 1)));
  api->tracepoint_maybe_do(client, "B1", 1, 1);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B2", 1, 0)));
  api->tracepoint_maybe_do(client, "B2", 1, 0);
  // println!("      are");
  printf("      are\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B5", 1, 0)));
  api->tracepoint_maybe_do(client, "B5", 1, 0);
  // println!("      server");
  printf("      server\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("B6", 1, 0)));
  api->tracepoint_maybe_do(client, "B6", 1, 0);
  api->finish(client_and_jh);
}
