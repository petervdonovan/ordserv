1. **$((e_1 \text{ is Receiving LTC})) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } e_1) < (\text{Tag } e_2)) \land (e_2 \text{ is Receiving LTC}) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is a receiving LTC event, the federate of $e_1$ is the same as the federate of $e_2$, and the Tag of $e_1$ is less than the Tag of $e_2$, and $e_2$ is also a receiving LTC event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

2. **$\text{FIRST }(((e_1 \text{ is Sending STOP\_GRN}) \lor (e_1 \text{ is Receiving LTC}) \lor (((e_1 \text{ is Receiving NET}) \lor (e_1 \text{ is Sending TAGGED\_MSG})))) \land (\text{Tag } e_1) = (\text{Tag } e_2) \land ((\text{Tag } e_1) \text{ finite}) \land (\text{Tag } e_1) \neq 0)) \land (((e_2 \text{ is Sending TAG}) \lor (e_2 \text{ is Sending PTAG}))) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is the first occurrence of sending STOP_GRN, receiving LTC, receiving NET, or sending TAGGED_MSG, and the Tag of $e_1$ is the same as the Tag of $e_2$, finite, and not zero, and $e_2$ is the first occurrence of sending TAG or sending PTAG, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

3. **$((e_1 \text{ is Receiving PORT\_ABS}) \lor (e_1 \text{ is Receiving TAGGED\_MSG}))) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } e_1) \leq (\text{Tag } e_2)) \land ((e_2 \text{ is Receiving LTC}))) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is a receiving PORT_ABS or receiving TAGGED_MSG event, the federate of $e_1$ is the same as the federate of $e_2$, the Tag of $e_1$ is less than or equal to the Tag of $e_2$, and $e_2$ is a receiving LTC event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

4. **$((e_1 \text{ is Receiving NET})) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } e_1) \leq (\text{Tag } e_2)) \land ((e_2 \text{ is Receiving LTC})) \land (\text{Tag } e_2) \neq 0) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is a receiving NET event, the federate of $e_1$ is the same as the federate of $e_2$, the Tag of $e_1$ is less than or equal to the Tag of $e_2$, $e_2$ is a receiving LTC event, and the Tag of $e_2$ is non-zero, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

5. **$((e_1 \text{ is Receiving LTC})) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } + \text{LargestDelayFromPreceding } e_1) < (\text{Tag } e_2)) \land ((e_2 \text{ is Receiving PORT\_ABS})) \lor (e_2 \text{ is Receiving TAGGED\_MSG})) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is a receiving LTC event, the federate of $e_1$ is the same as the federate of $e_2$, the sum of the Tag and LargestDelayFromPreceding of $e_1$ is less than the Tag of $e_2$, and $e_2$ is a receiving PORT_ABS or receiving TAGGED_MSG event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

6. **$\text{FIRST }(((e_1 \text{ is Sending TAG}) \lor (e_1 \text{ is Sending PTAG})) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } + \text{LargestDelayFromPreceding } e_1) \geq (\text{Tag } e_2))) \land (((e_2 \text{ is Receiving PORT\_ABS}) \lor (e_2 \text{ is Receiving TAGGED\_MSG})))) \land \lnot (\text{Fed } e_2 \text{ has no upstream with delay } \leq (\text{Tag } e_2)) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is the first occurrence of sending TAG or sending PTAG, the federate of $e_1$ is the same as the federate of $e_2$, and the sum of the Tag and LargestDelayFromPreceding of $e_1$ is greater than or equal to the Tag of $e_2$, and $e_2$ is a receiving PORT_ABS or receiving TAGGED_MSG event, and the federate of $e_2$ has an upstream with delay less than or equal to the Tag of $e_2$, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

7. **$((e_1 \text{ is Sending PTAG}) \lor (e_1 \text{ is Sending TAG})) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } e_1) < (\text{Tag } e_2)) \land ((e_2 \text{ is Sending PTAG}) \lor (e_2 \text{ is Sending TAG})) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is a sending PTAG or sending TAG event, the federate of $e_1$ is the same as the federate of $e_2$, the Tag of $e_1$ is less than the Tag of $e_2$, and $e_2$ is a sending PTAG or sending TAG event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

8. **$((e_1 \text{ is Sending PTAG})) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } e_1) \leq (\text{Tag } e_2)) \land (e_2 \text{ is Sending TAG}) \Rightarrow e_1 \prec e_2$:**

   - If $e_1$ is a sending PTAG event, the federate of $e_1$ is the same as the federate of $e_2$, the Tag of $e_1$ is less than or equal to the Tag of $e_2$, and $e_2$ is a sending TAG event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

9. **$\text{FedwiseFIRST }(((e_1 \text{ is Receiving LTC})) \land (\text{Federate of } e_1 \text{ is upstream of federate of } e_2 \text{ via a zero delay connection}) \land (\text{Tag } e_1) \geq (\text{Tag } e_2)) \lor (((e_1 \text{ is Sending TAG}) \lor (e_1 \text{ is Receiving NET}) \lor (e_1 \text{ is Sending STOP\_GRN}))) \land (\text{Federate of } e_1 \text{ is upstream of federate of } e_2 \text{ via a zero-delay connection}) \land (\text{Tag } e_1) \geq (\text{Tag } e_2)))) \land ((e_2 \text{ is Sending TAG})) \land (\text{Tag } e_2) \neq 0) \Rightarrow e_1 \prec e_2$:**

- If $e_1$ is the first occurrence of receiving LTC, the federate of $e_1$ is upstream of the federate of $e_2$ via a zero-delay connection, and the Tag of $e_1$ is greater than or equal to the Tag of $e_2$, or $e_1$ is the first occurrence of sending TAG, receiving NET, or sending STOP_GRN, the federate of $e_1$ is upstream of the federate of $e_2$ via a zero-delay connection, and the Tag of $e_1$ is greater than or equal to the Tag of $e_2$, and $e_2$ is a sending TAG event with a non-zero Tag, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

10. **$\text{FIRST }(((e_1 \text{ is Sending PTAG}) \land (\text{Federate of } e_1 \text{ is upstream of federate of } e_2 \text{ via a zero-delay connection}) \land (\text{Tag } e_1) = (\text{Tag } e_2)) \lor (((e_1 \text{ is Receiving NET}) \lor (e_1 \text{ is Sending STOP\_GRN}))) \land (\text{Federate}(e_1) = \text{Federate}(e_2) \lor (\text{Federate of } e_1 \text{ is directly upstream of federate of } e_2)) \land (\text{Tag } e_1) = (\text{Tag } e_2)))) \land ((e_2 \text{ is Sending PTAG})) \land (\text{Tag } e_2) \neq 0) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is the first occurrence of sending PTAG, the federate of $e_1$ is upstream of the federate of $e_2$ via a zero-delay connection, and the Tag of $e_1$ is equal to the Tag of $e_2$, or $e_1$ is the first occurrence of receiving NET or sending STOP_GRN, and the federate of $e_1$ is the same as the federate of $e_2$ or the federate of $e_1$ is directly upstream of the federate of $e_2$, and the Tag of $e_1$ is equal to the Tag of $e_2$, and $e_2$ is a sending PTAG event with a non-zero Tag, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

11. **$\text{FIRST }((e_1 \text{ is Receiving PORT\_ABS}) \land (\text{Federate of } e_1 \text{ is upstream of federate of } e_2 \text{ via a zero-delay connection}) \land (\text{Tag } e_1) = (\text{Tag } e_2))) \land (e_2 \text{ is Sending PORT\_ABS}) \Rightarrow e_1 \prec e_2$:**

- If $e_1$ is the first occurrence of receiving PORT_ABS, the federate of $e_1$ is upstream of the federate of $e_2$ via a zero-delay connection, and the Tag of $e_1$ is equal to the Tag of $e_2$, and $e_2$ is a sending PORT_ABS event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

12. **$\text{FIRST }((e_1 \text{ is Receiving TAGGED\_MSG}) \land (\text{Federate of } e_1 \text{ is directly upstream of federate of } e_2) \land (\text{Tag } e_1) = (\text{Tag } e_2))) \land (e_2 \text{ is Sending TAGGED\_MSG}) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is the first occurrence of receiving TAGGED_MSG, the federate of $e_1$ is directly upstream of the federate of $e_2$, and the Tag of $e_1$ is equal to the Tag of $e_2$, and $e_2$ is a sending TAGGED_MSG event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

13. **$((e_1 \text{ is Receiving PORT\_ABS}) \lor (e_1 \text{ is Receiving TAGGED\_MSG}))) \land \text{Federate}(e_1) = \text{Federate}(e_2) \land (\text{Tag } e_1) \leq (\text{Tag } e_2)) \land ((e_2 \text{ is Receiving LTC}))) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is a receiving PORT_ABS or receiving TAGGED_MSG event, the federate of $e_1$ is the same as the federate of $e_2$, the Tag of $e_1$ is less than or equal to the Tag of $e_2$, and $e_2$ is a receiving LTC event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

14. **$(e_1 \text{ is Receiving FED\_ID}) \land \text{Federate}(e_1) = \text{Federate}(e_2)) \land (e_2 \text{ is Sending ACK}) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is a receiving FED_ID event, the federate of $e_1$ is the same as the federate of $e_2$, and $e_2$ is a sending ACK event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

15. **$(e_1 \text{ is Sending ACK}) \land \text{Federate}(e_1) = \text{Federate}(e_2)) \land (e_2 \text{ is Receiving TIMESTAMP}) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is a sending ACK event, the federate of $e_1$ is the same as the federate of $e_2$, and $e_2$ is a receiving TIMESTAMP event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

16. **$(e_1 \text{ is Receiving TIMESTAMP}) \land \text{Federate}(e_1) = \text{Federate}(e_2)) \land (e_2 \text{ is Sending TIMESTAMP}) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is a receiving TIMESTAMP event, the federate of $e_1$ is the same as the federate of $e_2$, and $e_2$ is a sending TIMESTAMP event, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

17. **$(e_1 \text{ is Sending TIMESTAMP}) \land \text{Federate}(e_1) = \text{Federate}(e_2)) \land ((e_2 \text{ is Receiving NET})) \land \lnot ((\text{Tag } e_2) \neq 0) \Rightarrow e_1 \prec e_2$:**

    - If $e_1$ is a sending TIMESTAMP event, the federate of $e_1$ is the same as the federate of $e_2$, $e_2$ is a receiving NET event, and the Tag of $e_2$ is not non-zero, then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

18. **$(e_1 \text{ is Receiving TIMESTAMP})) \land ((e_2 \text{ is Receiving LTC}) \lor (e_2 \text{ is Receiving PORT\_ABS}) \lor (e_2 \text{ is Receiving TAGGED\_MSG}) \lor (e_2 \text{ is Sending TAG}) \lor (e_2 \text{ is Sending PTAG}) \lor (e_2 \text{ is Sending PORT\_ABS}) \lor (e_2 \text{ is Sending TAGGED\_MSG}) \lor (e_2 \text{ is Sending STOP\_GRN}) \lor (e_2 \text{ is Sending STOP\_REQ}) \lor (e_2 \text{ is Receiving STOP\_REQ}) \lor (e_2 \text{ is Receiving STOP\_REQ\_REP}))) \Rightarrow e_1 \prec e_2$:**
    - If $e_1$ is a receiving TIMESTAMP event, and $e_2$ is any of the listed events (receiving LTC, receiving PORT_ABS, receiving TAGGED_MSG, sending TAG, sending PTAG, sending PORT_ABS, sending TAGGED_MSG, sending STOP_GRN, sending STOP_REQ, receiving STOP_REQ, receiving STOP_REQ_REP), then it implies that $e_1$ cannot occur after $e_2$ in any execution of the federated program.

These propositions express ordering constraints between different events in a federated program, specifying the conditions under which one event cannot occur after another.
