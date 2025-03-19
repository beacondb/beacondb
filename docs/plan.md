# the beacondb plan 🔥🔥🔥🔥

This document aims to:

- provide context regarding the problem beaconDB aims to solve with publishing obfuscated data, as well as why the project is in this position of promising open data but not publishing it just yet
- propose a specific idea as "the plan" for publishing beaconDB data, explaining why this was chosen over other related ideas

## first, some context

When Mozilla Location Services shutdown in March, all crowdsourced WiFi geolocation data was made completely unavailable to the public. **Data that users had submitted was completely gone, and anyone who wanted to start up an alternative service would now be completely starting from scratch**.

Following the [MLS shutdown announcement](https://github.com/mozilla/ichnaea/issues/2065), there was lots of discussion from the open source community regarding next steps. Comments on the announcement show that people seem to be keen to support an alternative that _somehow_ publishes data, mainly to avoid becoming dependent on a centralised service for the second time.

That thread eventually got locked due to some heated discussion, and after almost a month I wasn't able to find another place where people had gathered to continue working towards a solution for publishing data. I wasn't even able to find a project that had started collecting data to be used in an MLS replacement, so I ended up starting beaconDB with two goals in mind:

- being a successor to MLS, by starting to receive data
- publishing collected data, after discussing possible solutions as a community.the main thing here is to try and strike a balance between utility and privacy that people are happy with - which can only be done if enough people participate in discussion.

10 months later, beaconDB is now sitting on more than 30 million WiFi APs (!!!) without having "a plan" to actually publish any data. While not ideal, I hope that at least explains how we got in this position :P.

## proposed plan

Goals:

- data must not have significant privacy impacts on AP owners
- data must not have significant security impacts on AP hardware
- data must not have significant privacy impacts on contributors
- _optout and _nomap must be filtered server side. while clients are expected to filter these APs before uploading, the server should not blindly trust that clients do this, and must still receive enough data to be able to remove these APs.
- obfuscation should not interfere with geolocation accuracy
- obfuscation should not interfere with the ability for a third-party to "fork" the database
- obfuscation should not be so complicated / compute intensive that it is easier to start from scratch instead of using beaconDB's data

Overview + details of proposed obfuscation:

1. A beaconhash refers to a SHA256 hash generated from an AP's MAC/BSSID and SSID, along with a salt.
   - for example, `salt_12:34:56:ab:cd:ef_My WiFi SSID` => `20b8c20d1d57e4bc8c742e273d7dd810331f1d7dce2ad38f60101c3de6d6a796`
   - not sure if SHA256 is the right choice for this, please let me know if you think something else might be better!
   - as the SSID can easily be set by AP owners, it adds more entropy to the beaconhash, making it quite computationally intensive to bruteforce.
2. APs are publicly identified by the first half of its beaconhash. This is a stable, globally unique identifier.
   - previous approaches to obfuscation used the idea of truncating hashes to reduce how identifiable an AP is, making it more difficult to track a single AP overtime. unfortunately, this idea would make it significantly harder for clients to estimate their location, while only making it slightly more difficult for stalkers to identify an AP in a small area like a city, as they would still be given a few possible locations.
   - beaconDB instead will take a simplistic approach to prevent AP tracking, by blocking an AP that has moved from being published until it has been confirmed as stable for at least two years. (the exact duration will need to be researched, once beaconDB has historical metrics)
   - as the beaconhash is based off the MAC + SSID of an AP, people worried about tracking can change their SSID to get a completely different beaconhash. changing AP MAC addresses to prevent tracking is not easy to do, and on some hardware may require rooting/custom firmware.
   - **after (at least) two years, a previously blocked AP will be published again. people may be able to lookup its old location in old/archived versions of public data dumps. if this concerns an AP owner, it is expected that they will change their SSID to get a new beaconhash, or add _optout/_nomap.** hopefully, this will raise public's awareness of tracking identifiers like this over time.
3. An AP's published location is pseudorandomly offset by 500m, derived using 16 bytes (2x f64) from the latter half of the AP's beaconhash.
   - counter-intuitively, this is not to protect the location of an AP. this this is to protect the location history of beaconDB's contributors.
   - beaconDB's current map only shows submissions using resolution 8 H3 cells, [which have an average area of 737m^2](https://h3geo.org/docs/core-library/restable/#average-area-in-m2). an offset of 500m is chosen as this closely matches the current map resolution, as an AP's public/obfuscated location will then always be in a 1km^2 area centered on it's real location. (500m in either direction = 1km x 1km)
   - similar to how the map will likely have an increased resolution in the future, if deemed safe to do so in terms of privacy, this offset could be reduced in future data dumps.
4. The last 16 bytes of the beaconhash are not used.
