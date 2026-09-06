// TypeSpec authority index for gha-indie-worker-interfaces.
//
// This file exists so `tsp compile contracts/typespec --no-emit` type-checks
// every slice at once. It is NOT the parity input: ores-contracts parses one
// slice file at a time (contracts/config/<slice>.config.json), because the
// toolkit's parser is deliberately single-file and does not follow imports.
//
// Neither authority is generated from the other. contracts/json-schema/<slice>.schema.json
// is the peer authority for each file imported below.

import "./ores.tsp";

import "./identity.tsp";
import "./onboarding.tsp";
import "./runs.tsp";
import "./workers.tsp";
import "./webhooks.tsp";
import "./chat.tsp";
import "./embeddings.tsp";
import "./sync.tsp";
import "./transport.tsp";
import "./errors.tsp";
