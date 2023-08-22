import detectEthereumProvider from '@metamask/detect-provider';
import { BrowserProvider, JsonRpcError, JsonRpcSigner } from 'ethers';
import { ReactNode, useCallback, useEffect, useRef, useState } from 'react';

import { Web3Context } from './web3Context';

type Props = {
  children: ReactNode;
};

export const Web3ContextProvider = ({ children }: Props) => {
  const initRef = useRef(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [provider, setProvider] = useState<BrowserProvider | undefined>();
  const [signer, setSigner] = useState<JsonRpcSigner | undefined>();
  const [chainId, setChainId] = useState<number | undefined>();
  const [address, setAddress] = useState<string | undefined>();
  const [isConnected, setConnected] = useState<boolean>(false);

  const connect = useCallback(async () => {
    const detected = await detectEthereumProvider({ mustBeMetaMask: true, silent: true });
    if (detected && window.ethereum) {
      setIsConnecting(true);
      try {
        const accounts = await (window.ethereum.request({
          method: 'eth_requestAccounts',
        }) as Promise<string[]>);
        if (Array.isArray(accounts) && accounts && accounts.length) {
          setAddress(accounts[0]);
          setConnected(true);
          const cId = await window.ethereum.request({ method: 'eth_chainId' });
          const id = parseInt(cId, 16);
          setChainId(id);
          setIsConnecting(false);
          const p = new BrowserProvider(window.ethereum);
          const s = await p.getSigner();
          setProvider(p);
          setSigner(s);
          return Promise.resolve({
            address: accounts[0],
            chainId: id,
          });
        }
      } catch (e) {
        setIsConnecting(false);
        return Promise.reject(e as JsonRpcError);
      }
    }
    return Promise.reject('Metamask not detected');
  }, []);

  const handleAccountsChange = useCallback((accounts: string[]) => {
    const assignHandlers = async () => {
      if (window.ethereum) {
        const p = new BrowserProvider(window.ethereum);
        const s = await p.getSigner();
        const cId = await window.ethereum.request({
          method: 'eth_chainId',
        });
        const id = parseInt(cId, 16);
        setChainId(id);
        setProvider(p);
        setSigner(s);
      }
    };

    if (accounts.length) {
      setAddress(accounts[0]);
      setConnected(true);
      assignHandlers();
    }

    // if list is empty user removed permissions to he's wallet
    if (accounts.length === 0) {
      setConnected(false);
      setAddress(undefined);
      setProvider(undefined);
      setSigner(undefined);
      setChainId(undefined);
    }
  }, []);

  // detect connected MM on mount
  useEffect(() => {
    const detectPermissions = async () => {
      const detected = await detectEthereumProvider({
        mustBeMetaMask: true,
        silent: true,
      });
      if (detected && window.ethereum && window.ethereum.selectedAddress) {
        window.ethereum
          ?.request({ method: 'eth_requestAccounts' })
          .then(async (accounts: string[]) => {
            if (accounts.length) {
              setAddress(accounts[0]);
              setConnected(true);
              if (window.ethereum) {
                const cId = await window.ethereum.request({
                  method: 'eth_chainId',
                });
                const id = parseInt(cId, 16);
                setChainId(id);
                setIsConnecting(false);
                const p = new BrowserProvider(window.ethereum);
                const s = await p.getSigner();
                setProvider(p);
                setSigner(s);
              }
            }
          });
      }
    };
    detectPermissions();
  }, []);

  // watch for events
  useEffect(() => {
    const handleChainChange = () => {
      window.location.reload();
    };
    const init = async () => {
      const detected = await detectEthereumProvider({
        mustBeMetaMask: true,
        silent: true,
      });
      if (detected && window.ethereum) {
        await window.ethereum.on('accountsChanged', handleAccountsChange);
        await window.ethereum.on('chainChanged', handleChainChange);
      }
      initRef.current = false;
    };

    if (!initRef.current) {
      initRef.current = true;
      init();
    }

    return () => {
      window.ethereum?.removeListener('accountsChanged', handleAccountsChange);
      window.ethereum?.removeListener('chainChanged', handleChainChange);
    };
  }, [handleAccountsChange]);

  return (
    <Web3Context.Provider
      value={{ chainId, address, isConnected, signer, provider, connect, isConnecting }}
    >
      {children}
    </Web3Context.Provider>
  );
};
