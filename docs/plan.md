# the beacondb plan 🔥🔥🔥🔥

This document aims to:

- provide context regarding the problem beaconDB aims to solve with publishing obfuscated data
- explain why this data should be obfuscated
- propose a specific idea as "the plan" for publishing beaconDB data

## first, some context

When Mozilla Location Services shutdown in March, all crowdsourced WiFi geolocation data was made completely unavailable to the public. **Data that users had submitted was completely gone, and anyone who wanted to start up an alternative service would now be completely starting from scratch**.

Following the [MLS shutdown announcement](https://github.com/mozilla/ichnaea/issues/2065), there was lots of discussion from the open source community regarding next steps. I'll reference some comments in greater detail down below, but people in that thread seem to be quite keen on seeing an alternative emerge that _somehow_ publishes data, mainly avoid becoming dependent on a centralised service for the second time.

That thread eventually got locked due to some heated discussion, and after almost a month I wasn't able to find another place where people had gathered to continue working towards a solution for publishing data. I wasn't even able to find anyone who had started collecting data that could eventually be used in an MLS replacement, so I ended up starting beaconDB with two goals in mind:

- being a sucessor to MLS, by starting to receive data
- publishing collected data, after discussing possible solutions as a community, and making sure anyone with a better idea is giving the

10 months later, beaconDB is now sitting on more than 30 million WiFi APs without having "a plan" to actually publish any data. While less than ideal, I hope that explains how we got in this awkward position :P.

## utility vs privacy

## the plan
