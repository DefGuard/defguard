import { Provider, Signer } from 'ethers';
import { createContext } from 'react';

export const Web3Context = createContext<Web3ContextType | undefined>(undefined);

export type Web3ContextType = {
  isConnected: boolean;
  isConnecting?: boolean;
  chainId?: number;
  address?: string;
  provider?: Provider;
  signer?: Signer;
  connect: () => Promise<{
    address: string;
  }>;
};
