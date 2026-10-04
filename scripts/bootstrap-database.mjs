#!/usr/bin/env node
// Bootstrap entrypoint placeholder: delegates to the platform database CLI
// once the sdkwork-database workspace link is registered for this module.
import process from 'node:process';

console.error('compute db bootstrap: pending sdkwork-database workspace registration');
process.exitCode = 1;
