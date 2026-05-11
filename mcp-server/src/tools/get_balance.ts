import { z } from "zod";
import { ApiClient } from "../api.js";

const DEFAULT_SOLANA_RPC = "https://api.mainnet-beta.solana.com";

const getBalanceInput = z.object({
  account: z.string(),
  token_mint: z.string().optional(),
});

interface SolanaRpcResponse<T> {
  jsonrpc: string;
  id: number;
  result: T;
  error?: { code: number; message: string };
}

interface GetBalanceResult {
  value: number;
  context: { slot: number };
}

interface TokenAccountsByOwnerResult {
  value: Array<{
    pubkey: string;
    account: {
      data: { parsed: { info: { tokenAmount: { amount: string; decimals: number; uiAmount: number } } } };
      lamports: number;
    };
  }>;
}

async function solanaRpc<T>(method: string, params: unknown[]): Promise<SolanaRpcResponse<T>> {
  const rpcUrl = process.env.SOLANA_RPC_URL ?? DEFAULT_SOLANA_RPC;
  const res = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`Solana RPC ${res.status}: ${text || res.statusText}`);
  }
  const json = await res.json() as SolanaRpcResponse<T>;
  if (json.error) {
    throw new Error(`Solana RPC error ${json.error.code}: ${json.error.message}`);
  }
  return json;
}

export const tool = {
  name: "sk.get_balance",
  description: "Read SOL or SPL token balance for any Solana account",
  inputSchema: {
    type: "object",
    required: ["account"],
    properties: {
      account: { type: "string", description: "Base58 Solana account address" },
      token_mint: { type: "string", description: "SPL token mint address; omit for SOL balance" },
    },
  } as const,
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  handler: async (args: unknown, _api: ApiClient) => {
    const parsed = getBalanceInput.parse(args);

    if (!parsed.token_mint) {
      // SOL balance
      const resp = await solanaRpc<GetBalanceResult>("getBalance", [parsed.account]);
      return {
        account: parsed.account,
        lamports: resp.result.value,
        sol: resp.result.value / 1e9,
        slot: resp.result.context.slot,
      };
    } else {
      // SPL token balance
      const resp = await solanaRpc<TokenAccountsByOwnerResult>(
        "getTokenAccountsByOwner",
        [
          parsed.account,
          { mint: parsed.token_mint },
          { encoding: "jsonParsed" },
        ],
      );
      const accounts = resp.result.value;
      if (accounts.length === 0) {
        return {
          account: parsed.account,
          token_mint: parsed.token_mint,
          amount: "0",
          decimals: 0,
          ui_amount: 0,
        };
      }
      const tokenAmount = accounts[0].account.data.parsed.info.tokenAmount;
      return {
        account: parsed.account,
        token_mint: parsed.token_mint,
        token_account: accounts[0].pubkey,
        amount: tokenAmount.amount,
        decimals: tokenAmount.decimals,
        ui_amount: tokenAmount.uiAmount,
      };
    }
  },
};
