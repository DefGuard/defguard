import { useContext } from 'react';
import { Web3Context } from '../Web3ContextProvider/web3Context';

export const useWeb3Connection = () => {
  const contextData = useContext(Web3Context);

  if (!contextData) return {};

  const { isConnected, connect } = contextData;

  return {
    isConnected,
    connect,
  };
};
