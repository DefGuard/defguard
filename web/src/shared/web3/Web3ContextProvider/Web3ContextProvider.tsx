import { BrowserProvider, JsonRpcError, Provider, Signer } from 'ethers';
import { ReactNode, useCallback, useEffect, useState } from 'react';

import { Web3Context } from './web3Context';

const ethereum = window.ethereum;

type Props = {
  children: ReactNode;
};

export const Web3ContextProvider = ({ children }: Props) => {
  const [provider, setProvider] = useState<Provider | undefined>();
  const [signer, setSigner] = useState<Signer | undefined>();
  const [chainId, setChainId] = useState<number | undefined>();
  const [address, setAddress] = useState<string | undefined>();
  const [isConnected, setConnected] = useState<boolean>(false);

  const connect = useCallback(async () => {
    if (ethereum?.isMetaMask && provider) {
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
        }
      } catch (e) {
        return Promise.reject(e as JsonRpcError);
      }
    }
    return Promise.reject('No ethereum in window');
  }, [provider]);

  useEffect(() => {
    if (provider) {
      const setupListeners = async () => {
        provider.on('connect', async () => {
          setConnected(true);
        });

        provider.on('disconnect', async () => {
          setConnected(false);
        });

        provider.on('accountsChanged', async (accounts: string[]) => {
          if (accounts && accounts.length) {
            setAddress(accounts[0]);
          } else {
            setChainId(undefined);
            setAddress(undefined);
            setConnected(false);
          }
        });

        provider.on('chainChanged', async () => {
          window.location.reload();
        });
      };

      setupListeners();

      return () => {
        provider.removeAllListeners('');
      };
    }
  }, [provider]);

  useEffect(() => {
    if (ethereum?.isMetaMask) {
      const init = async () => {
        const p = new BrowserProvider(ethereum);
        const s = await p.getSigner();
        setProvider(p);
        setSigner(s);
      };
      init();
    }
  }, []);

  return (
    <Web3Context.Provider
      value={{ chainId, address, isConnected, signer, provider, connect }}
    >
      {children}
    </Web3Context.Provider>
  );
};
