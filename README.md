# beacondb

[beaconDB](https://beacondb.net/) aims to be an alternative to Mozilla Location Services that offers public domain dumps of its WiFi database.

When [Mozilla Location Services](https://location.services.mozilla.com/) [shut down](https://github.com/mozilla/ichnaea/issues/2065), it wasn't able to publish the massive amount of access points its users had collected due to legal and privacy concerns. beaconDB's use of [beacon hashes](/docs/beacon-hashes.md) makes it possible for the public to lookup valuable location data while preventing bad actors from taking advantage of such a large dataset. By hashing BSSIDs and SSIDs, sensitive information that could be used to identify people or vulnerable hardware is removed, resulting in a database that can be redistributed without such concerns.
