import { useContext } from 'react';

import { Web3Context } from '../Web3ContextProvider/web3Context';

export const useWeb3Account = () => {
  const contextData = useContext(Web3Context);

  if (!contextData) return {};

  const { address, chainId } = contextData;

  return {
    address,
    chainId,
  };
};
