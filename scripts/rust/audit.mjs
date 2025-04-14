#!/usr/bin/env zx
import "zx/globals";

const advisories = [
    // ed25519-dalek: Double Public Key Signing Function Oracle Attack
    //
    // Remove once repo upgrades to ed25519-dalek v2
    "RUSTSEC-2022-0093",

    // curve25519-dalek
    //
    // Remove once repo upgrades to curve25519-dalek v4
    "RUSTSEC-2024-0344",

    // Crate:     tonic
    // Version:   0.9.2
    // Title:     Remotely exploitable Denial of Service in Tonic
    // Date:      2024-10-01
    // ID:        RUSTSEC-2024-0376
    // URL:       https://rustsec.org/advisories/RUSTSEC-2024-0376
    // Solution:  Upgrade to >=0.12.3
    "RUSTSEC-2024-0376",

    // Crate:     ring
    // Version:   0.16.20
    // Title:     Some AES functions may panic when overflow checking is enabled.
    // Date:      2025-03-06
    // ID:        RUSTSEC-2025-0009
    // URL:       https://rustsec.org/advisories/RUSTSEC-2025-0009
    // Solution:  Upgrade to >=0.17.12
    // Dependency tree:
    // ring 0.16.20
    //
    // Crate:     ring
    // Version:   0.17.3
    // Title:     Some AES functions may panic when overflow checking is enabled.
    // Date:      2025-03-06
    // ID:        RUSTSEC-2025-0009
    // URL:       https://rustsec.org/advisories/RUSTSEC-2025-0009
    // Solution:  Upgrade to >=0.17.12
    // Dependency tree:
    // ring 0.17.3
    "RUSTSEC-2025-0009",

    // Crate:     openssl
    // Version:   0.10.71
    // Title:     Use-After-Free in `Md::fetch` and `Cipher::fetch`
    // Date:      2025-04-04
    // ID:        RUSTSEC-2025-0022
    // URL:       https://rustsec.org/advisories/RUSTSEC-2025-0022
    // Solution:  Upgrade to >=0.10.72
    "RUSTSEC-2025-0022",

    // Crate:     crossbeam-channel
    // Version:   0.5.14
    // Title:     crossbeam-channel: double free on Drop
    // Date:      2025-04-08
    // ID:        RUSTSEC-2025-0024
    // URL:       https://rustsec.org/advisories/RUSTSEC-2025-0024
    // Solution:  Upgrade to >=0.5.15
    "RUSTSEC-2025-0024",
];
const ignores = [];
advisories.forEach((x) => {
    ignores.push("--ignore");
    ignores.push(x);
});

// Check Solana version.
await $`cargo audit ${ignores}`;
