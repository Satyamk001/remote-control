# Phase 16 — Reliability + Failure Testing
Implement Phase 16 only.

Perform systematic reliability testing.

Test:
1. LAN Windows → Ubuntu
2. LAN Ubuntu → Windows
3. Internet Windows → Friend Windows
4. Direct P2P
5. TURN relay
6. Host disconnect
7. Client disconnect
8. Network interruption
9. Wi-Fi switch
10. High latency
11. Packet loss
12. Bandwidth reduction
13. Host restart
14. Client restart
15. Display resolution change
16. Monitor disconnect
17. Remote application crash
18. WebRTC failure
19. Signaling server failure
20. TURN failure

Verify:
- graceful recovery
- reconnect
- no stuck input
- no stale sessions
- no leaked resources
- no orphaned threads
- no memory leaks
- safe cleanup

Create automated tests where practical.

Create a failure matrix documenting:
Failure
Detection
Recovery
User-visible behavior

STOP after Phase 16.
