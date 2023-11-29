#include "c-ordering-client.h"

#include <stdio.h>

int main(int argc, char **argv) {
  OrderingClientApi* api = load_ordering_client_api();
  printf("loaded api: %p\n", api);
  // let (client, _jh) = ordering_server::client::BlockingClient::start(("127.0.0.1", 15045), 0);
  ClientAndJoinHandle client_and_jh = api->start_client(0);
  void* client = client_and_jh.client;
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A99", 0, 0)));
  api->tracepoint_maybe_do(client, "A99", 0, 0);
  // println!("the");
  printf("the\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A0", 0, 0)));
  api->tracepoint_maybe_do(client, "A0", 0, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A1", 0, 0)));
  api->tracepoint_maybe_do(client, "A1", 0, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A1", 0, 1)));
  api->tracepoint_maybe_do(client, "A1", 0, 1);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A2", 0, 0)));
  api->tracepoint_maybe_do(client, "A2", 0, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A3", 0, 0)));
  api->tracepoint_maybe_do(client, "A3", 0, 0);
  // println!("by");
  printf("by\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("A4", 0, 0)));
  api->tracepoint_maybe_do(client, "A4", 0, 0);
  // client.tracepoint_maybe_wait(HookInvocation::from_short(("A4", 0, 1)));
  api->tracepoint_maybe_wait(client, "A4", 0, 1);
  // println!("ordering");
  printf("ordering\n");
  // client.tracepoint_maybe_notify(HookInvocation::from_short(("A4", 0, 1)));
  api->tracepoint_maybe_notify(client, "A4", 0, 1);
  api->finish(client_and_jh);
}
