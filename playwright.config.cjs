/** @type {import('@playwright/test').PlaywrightTestConfig} */
module.exports = {
  testDir: "./apps/desktop/e2e",
  timeout: 120_000,
  expect: { timeout: 20_000 },
  retries: 0,
  workers: 1,
  reporter: [["list"]],
};

