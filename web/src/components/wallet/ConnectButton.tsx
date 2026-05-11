"use client";
import { useWallet } from "@solana/wallet-adapter-react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { Btn } from "@/components/primitives/Btn";
import { formatAddress } from "@/lib/utils/format";

export function ConnectButton() {
  const { connected, publicKey, disconnect } = useWallet();
  const { setVisible } = useWalletModal();

  if (!connected || !publicKey) {
    return (
      <Btn size="sm" variant="default" onClick={() => setVisible(true)}>
        Connect
      </Btn>
    );
  }

  return (
    <Btn
      size="sm"
      variant="default"
      onClick={disconnect}
      title="Click to disconnect"
    >
      {formatAddress(publicKey.toBase58())}
    </Btn>
  );
}
