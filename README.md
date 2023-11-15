# Skaja

The name comes from translating what "Redis" stands for --Remote Dictionary Server--
into Indonesian, which becomes "Server Kamus Jarak Jauh" or Skaja for short.

Ikr, what a goofy ahh name.

## Communication Protocol

The client and server communicates over a custom binary protocol built on top of TCP.
The payload is divided into chunks with each chunk used to encode different kinds of data.
Below are the visualizations of the protocol:

### Request

![image](https://github.com/danilhendrasr/skaja/assets/45989466/be58f79b-4de8-43b2-bd9d-b8985d1227ba)

### Response

<img src="https://github.com/danilhendrasr/skaja/assets/45989466/21e3e213-86a3-4765-81f6-1109137c1821" width="500" />
