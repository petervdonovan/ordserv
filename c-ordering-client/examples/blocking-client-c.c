#include "c-ordering-client.h"

#include <stdio.h>

int main(int argc, char **argv) {
  OrderingClientApi* api = load_ordering_client_api();
  // let (client, _jh) = ordering_server::client::BlockingClient::start(("127.0.0.1", 15045), 1);
  ClientAndJoinHandle client_and_jh = api->start_client(-1);
  void* client = client_and_jh.client;
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C99", 2, 0)));
  api->tracepoint_maybe_do(client, "C99", -1, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C0", 2, 0)));
  api->tracepoint_maybe_do(client, "C0", -1, 0);
  // println!("            of");
  printf("            of\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C0", 2, 1)));
  api->tracepoint_maybe_do(client, "C0", -1, 1);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 0)));
  api->tracepoint_maybe_do(client, "C1", -1, 0);
  // println!("            sentence");
  printf("            sentence\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 1)));
  api->tracepoint_maybe_do(client, "C1", -1, 1);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 2)));
  api->tracepoint_maybe_do(client, "C1", -1, 2);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 3)));
  api->tracepoint_maybe_do(client, "C1", -1, 3);
  // println!("            ordered");
  printf("            ordered\n");
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C1", 2, 4)));
  api->tracepoint_maybe_do(client, "C1", -1, 4);
  // client.tracepoint_maybe_wait(HookInvocation::from_short(("C2", 2, 0)));
  api->tracepoint_maybe_wait(client, "C2", -1, 0);
  // println!("            the");
  printf("            the\n");
  // client.tracepoint_maybe_notify(HookInvocation::from_short(("C2", 2, 0)));
  api->tracepoint_maybe_notify(client, "C2", -1, 0);
  // client.tracepoint_maybe_do(HookInvocation::from_short(("C2", 2, 1)));
  api->tracepoint_maybe_do(client, "C2", -1, 1);
  // println!("            .");
  printf("            .\n");
  api->drop_join_handle(client_and_jh.join_handle);
}
