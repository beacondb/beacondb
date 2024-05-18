# Beacon hashes

Beacon hashes are verifiable identifiers for access points. By hashing access point information, any potentially sensitive data is removed. BeaconDB only publishes the first 56 of the 64 bytes in a beacon hash, as the last 8 are used to verify submitted data.

## Advantages

- access points with changing BSSIDs, such as personal mobile hotspots, cannot be tracked over time
- including SSIDs in the hash makes it easy for access point owners to invalidate previously recorded locations
- data that can be used to identify and locate potentially vulnerable access points is not published, such as vendor and encryption information
  - BSSIDs/MAC addresses are not completely random:
    - the [OUI](https://en.wikipedia.org/wiki/Organizationally_unique_identifier) leaks the vendor/manufacturer of an access point
    - the organization might assign the last three identifying octets sequentially, meaning that they could potentially be used to identify outdated models containing security flaws
- the database cannot be used to dox online users locations based on an SSID, which can easily be leaked when stopping screen recordings on iOS and Android as they are displayed in the quick settings menu
- sensitive content in SSIDs are redacted

## Disadvantages

- hashes use significantly more storage than MAC addresses (6 bytes vs 64 bytes for sha256)
- any software that submits or uses data must take care to ensure hashes are consistent across implementations and operating systems
- data quality issues are more difficult to identify

## Generation

A beacon hash is derived from an access point's BSSID and SSID, which is then hashed using sha256. They are formatted using:

```
beacon_{bssid}_{ssid}
```

For example, a mobile hotspot broadcasting a BSSID of `12:34:56:ab:cd:ef` and SSID of `AndroidAP` is formatted as `beacondb_12:34:56:ab:cd:ef_AndroidAP`. This can then be hashed using sha256:

```sh
$ echo -n "beacondb_12:34:56:ab:cd:ef_AndroidAP" | sha256sum
a3983c0740295e3da73d0ab0d5dc7dd8d9d7525b48fc264184daa6af530f4628  -
```

If the access point above had a hidden SSID, it would be formatted as `beacondb_12:34:56:ab:cd:ef_`. Note that the underscore seperator is still included at the end of the string.

In order for hashes to be consistent across implementations, software must ensure:

- appropriate testing using the official test dataset _todo_
- BSSIDs are formatted with colons and lowercase characters
- leading and trailing whitespace from SSIDs are removed
- SSIDs that are only made up of spaces or null characters are treated as hidden SSIDs

## Usage

Only the first 56 bytes of the beacon hash is published by BeaconDB and usable for client geolocation. BeaconDB uses the other 8 bytes to establish trust and prevent incorrect submissions.
