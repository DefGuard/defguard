import { BrowserProvider, JsonRpcError, JsonRpcSigner } from 'ethers';
import { isUndefined } from 'lodash-es';
import { ReactNode, useCallback, useEffect, useState } from 'react';

import { ConnectInfo } from './types';
import { Web3Context } from './web3Context';

const ethereum = window.ethereum;

type Props = {
  children: ReactNode;
};

export const Web3ContextProvider = ({ children }: Props) => {
  const [isConnecting, setIsConnecting] = useState(false);
  const [provider, setProvider] = useState<BrowserProvider | undefined>();
  const [signer, setSigner] = useState<JsonRpcSigner | undefined>();
  const [chainId, setChainId] = useState<number | undefined>();
  const [address, setAddress] = useState<string | undefined>();
  const [isConnected, setConnected] = useState<boolean>(false);

  const connect = useCallback(async () => {
    if (ethereum?.isMetaMask && provider) {
      setIsConnecting(true);
      try {
        const accounts = await (ethereum.request({
          method: 'eth_requestAccounts',
        }) as Promise<string[]>);
        if (Array.isArray(accounts) && accounts && accounts.length) {
          setAddress(accounts[0]);
          setConnected(true);

          const cId = await ethereum.request({ method: 'eth_chainId' });
          if (typeof cId === 'string') {
            setChainId(parseInt(cId, 16));
          }
          setIsConnecting(false);
          return Promise.resolve({
            address: accounts[0],
          });
        }
      } catch (e) {
        setIsConnecting(false);
        return Promise.reject(e as JsonRpcError);
      }
    }
    return Promise.reject('No ethereum in window');
  }, [provider]);

  const handleAccountsChange = useCallback((accounts: string[]) => {
    if (accounts.length) {
      console.log(accounts);
      setAddress(accounts[0]);
    }
  }, []);

  const handleConnect = useCallback((data: ConnectInfo) => {
    const { chainId } = data;
    setChainId(parseInt(chainId, 16));
    setConnected(true);
  }, []);

  const handleDisconnect = useCallback(() => {
    setConnected(false);
  }, []);

  useEffect(() => {
    if (ethereum && ethereum.isMetaMask) {
      ethereum.on('accountsChanged', handleAccountsChange);
      ethereum.on('connect', handleConnect);
      ethereum.on('disconnect', handleDisconnect);
      ethereum.on('chainChanged', () => window.location.reload());
      return () => {
        ethereum.removeListener('accountsChanged', handleAccountsChange);
        ethereum.removeListener('connect', handleConnect);
        ethereum.removeListener('disconnect', handleDisconnect);
      };
    }
  }, [handleAccountsChange, handleConnect, handleDisconnect]);

  useEffect(() => {
    if (ethereum && ethereum?.isMetaMask) {
      const init = async () => {
        const p = new BrowserProvider(ethereum);
        const s = await p.getSigner();
        setProvider(p);
        setSigner(s);
        if (!isUndefined(ethereum.isConnected)) {
          setConnected(ethereum.isConnected());
        }
      };
      init();
    }
  }, []);

  return (
    <Web3Context.Provider
      value={{ chainId, address, isConnected, signer, provider, connect, isConnecting }}
    >
      {children}
    </Web3Context.Provider>
  );
};
