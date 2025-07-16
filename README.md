# Ponder
Wifi enabled LG devices under the ThinQ line communicate through the cloud, this project implements the ThinQ server so that it can be run locally (integrated into Home Assistant) and the devices can be provisioned to use Ponder instead of LG's servers.

Ponder is starting out as a Rust rewrite of the amazing work done by [anszom](https://github.com/anszom) at [rethink](https://github.com/anszom/rethink), all credit for the reverse engineering work, documentation of the behavior and protocol, and of course the inspiration goes to them.

The goal is to build a performant and easily expandable implementation, hopefully expanding beyond air conditioners through community contributions.

# Status
Right now Ponder is a close rewrite of rethink, this gives me a starting point but it's not the end goal, here are a couple things I want to improve:
- Rewrite the architecture to be more Rusty
- Improve performance
- Reduce the internal MQTT broker or look into using the Home Assistant broker itself directly
- Fix bugs that were also present in rethink
- Write a macro for device definitions
- Package Ponder for home assistant/hass

### Why are you patching rmqtt-net?
I wanted to have this documented here because it wasn't written down anywhere else (wasn't a problem for rethink).

The wifi modem in the 2 air conditioners I have available to test use old TLS ciphersuites (CBC), these are not available in rustls due to security concerns, for this reason I had to patch rmqtt-net to use openssl instead of rustls, otherwise the ThinQ devices would always fail to connect as they require an SSL connection and only support CBC suites.
