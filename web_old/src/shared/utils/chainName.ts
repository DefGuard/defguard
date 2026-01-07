export interface Chain {
  name: string;
  network: string;
}

export const chains: Record<number, Chain> = {
  1: { name: 'Ethereum', network: 'Mainnet' },
  3: { name: 'Ethereum', network: 'Ropsten' },
  4: { name: 'Ethereum', network: 'Rinkeby' },
  5: { name: 'Ethereum', network: 'Goerli' },
  42: { name: 'Ethereum', network: 'Kovan' },
  11155111: { name: 'Ethereum', network: 'Sepolia' },
  10: { name: 'Optimism', network: 'Mainnet' },
  69: { name: 'Optimism', network: 'Kovan' },
  420: { name: 'Optimism', network: 'Goerli' },
  137: { name: 'Polygon', network: 'Mainnet' },
  80001: { name: 'Polygon', network: 'Mumbai' },
  42161: { name: 'Arbitrum', network: 'One' },
  421613: { name: 'Arbitrum', network: 'Goerli' },
  421611: { name: 'Arbitrum', network: 'Rinkeby' },
  1337: { name: 'Localhost', network: 'Local' },
  31337: { name: 'Hardhat', network: 'Local' },
};

export const chainName = (id: number): string | undefined => {
  const chain = chains[id];
  return chain ? `${chain.name} ${chain.network}` : undefined;
};
