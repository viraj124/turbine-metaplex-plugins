import { expect } from "chai";
import { LiteSVMTransactionError } from "./litesvm-provider";

export async function expectFailure(
  action: () => unknown | Promise<unknown>,
  expected: string,
): Promise<void> {
  try {
    await action();
  } catch (error) {
    const logs =
      error instanceof LiteSVMTransactionError
        ? error.logs
        : ((error as { logs?: string[] }).logs ?? []);
    expect([String(error), ...logs].join("\n")).to.include(expected);
    return;
  }

  expect.fail(`expected a failure mentioning "${expected}"`);
}
