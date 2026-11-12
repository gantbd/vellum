#!/usr/bin/env node

import { spawnSync } from "node:child_process";

const settings = [
  ["VELLUM_QUALIFICATION_SEED", 0x56454c4c554d0001n, 0xffffffffffffffffn],
  ["VELLUM_WIRE_CASES", 4096n, 1_000_000n],
  ["VELLUM_TRANSPORT_CASES", 2048n, 1_000_000n],
  ["VELLUM_SERVICE_CYCLES", 64n, 10_000n],
  ["VELLUM_DAEMON_CYCLES", 16n, 1_000n],
];

for (const [name, fallback, maximum] of settings) {
  const source = process.env[name] ?? fallback.toString();
  if (!/^[1-9][0-9]*$/.test(source)) {
    throw new Error(`${name} must be a positive decimal integer`);
  }
  const value = BigInt(source);
  if (value > maximum) {
    throw new Error(`${name} exceeds its explicit qualification limit`);
  }
  process.env[name] = source;
}

const qualifications = [
  ["vellum-wire", "deterministic_wire_mutation_is_panic_free_repeatable_and_canonical"],
  ["vellum-transport", "deterministic_frame_soak_preserves_bounds_truncation_and_one_exchange"],
];

if (process.platform === "darwin") {
  qualifications.push(
    ["vellum-service", "deterministic_service_soak_preserves_replay_pagination_and_removal_laws"],
    [
      "vellum-daemon",
      "bounded_daemon_soak_closes_valid_malformed_oversize_and_truncated_connections",
    ],
  );
} else {
  process.stdout.write(
    "fuzz-soak: service and daemon lifecycle qualification requires the macOS adapter profile\n",
  );
}

for (const [packageName, testName] of qualifications) {
  const result = spawnSync(
    "cargo",
    [
      "test",
      "--offline",
      "--locked",
      "-p",
      packageName,
      "--test",
      "qualification",
      "--",
      "--ignored",
      "--exact",
      testName,
    ],
    { env: process.env, stdio: "inherit" },
  );
  if (result.error) {
    throw new Error(`unable to run ${packageName} qualification`);
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

process.stdout.write(
  `fuzz-soak: deterministic bounded qualification passed (${process.env.VELLUM_WIRE_CASES} wire mutations, ${process.env.VELLUM_TRANSPORT_CASES} transport cases, ${process.env.VELLUM_SERVICE_CYCLES} service cycles, ${process.env.VELLUM_DAEMON_CYCLES} daemon cycles)\n`,
);
