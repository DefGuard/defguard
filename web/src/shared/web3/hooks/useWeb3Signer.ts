import { useContext } from 'react';
import { Web3Context } from '../Web3ContextProvider/web3Context';

export const useWeb3Signer = () => {
  const contextData = useContext(Web3Context);

  return {
    signer: contextData?.signer,
  };
};
