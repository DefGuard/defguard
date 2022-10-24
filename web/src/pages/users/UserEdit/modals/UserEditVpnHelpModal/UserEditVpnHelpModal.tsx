import './style.scss';

import AutoConfigImageOne from '@shared/images/png/auto-config-1.png';
import AutoConfigImageTwo from '@shared/images/png/auto-config-2.png';
import ManualConfigImageOne from '@shared/images/png/manual-config-1.png';
import clipboard from 'clipboardy';
import React from 'react';
import { useState } from 'react';
import { useEffect } from 'react';

import Button, {
  ButtonSize,
} from '../../../../../shared/components/layout/Button/Button';
import MessageBox, {
  MessageBoxType,
} from '../../../../../shared/components/layout/MessageBox/MessageBox';
import Modal from '../../../../../shared/components/layout/Modal/Modal';
import SvgIconCancel from '../../../../../shared/components/svg/IconCancel';

interface Props {
  isOpen: boolean;
  setIsOpen: (v: boolean) => void;
}

enum ConfigurationTypeEnum {
  Manual = 'manual',
  Automatic = 'automatic',
}

const wireguardDefaultConfig = `
[Interface]
PrivateKey = YOUR_PRIVATE_KEY=
Address = 10.6.0.19/32
DNS = 10.4.0.1, 10.4.0.2

[Peer]
PublicKey = YOUR_PUBLIC_KEY=
AllowedIPs = 10.4.0.0/24, 10.50.0.0/24, 10.7.0.0/16
Endpoint = vpn.teonite.net:7301
PersistentKeepalive = 25
`;

const UserEditVpnHelpModal: React.FC<Props> = ({ isOpen, setIsOpen }) => {
  const [configurationType, setConfigurationType] =
    useState<ConfigurationTypeEnum>(ConfigurationTypeEnum.Automatic);
  const [copied, setCopied] = useState<boolean>(false);

  const handleCopy = () => {
    clipboard.write(wireguardDefaultConfig).then(() => setCopied(true));
  };

  useEffect(() => {
    if (copied) {
      setTimeout(() => setCopied(false), 1000);
    }
  }, [copied]);

  return (
    <Modal
      setIsOpen={setIsOpen}
      isOpen={isOpen}
      backdrop
      className="vpn-configuration middle"
    >
      <div className="white-middle-modal vpn-help">
        <h3>
          WireGuard VPN configuration{' '}
          <button
            className="icon-button close-modal"
            onClick={() => setIsOpen(false)}
          >
            <SvgIconCancel />
          </button>
        </h3>
        <MessageBox message="This short tutorial will help you to setup your VPN connection using DefGuard and WireGuard." />
        <div className="controls">
          <button
            className={
              configurationType === ConfigurationTypeEnum.Automatic
                ? 'active'
                : ''
            }
            onClick={() =>
              setConfigurationType(ConfigurationTypeEnum.Automatic)
            }
          >
            <span>Automatic configuration</span>
          </button>
          <button
            className={
              configurationType === ConfigurationTypeEnum.Manual ? 'active' : ''
            }
            onClick={() => setConfigurationType(ConfigurationTypeEnum.Manual)}
          >
            <span>Manual configuration</span>
          </button>
        </div>
        {configurationType === ConfigurationTypeEnum.Automatic ? (
          <>
            <div className="step">
              <h5>Step 1:</h5>
              <p>
                Add a new device in your account settings using the
                &quot;automatic configuration&quot; setup and download the
                configuration file to your hard drive.
              </p>
            </div>
            <div className="step">
              <h5>Step 2:</h5>
              <p>Download and install WireGuard client.</p>
              <Button
                className="small"
                text="Download Wireguard"
                size={ButtonSize.SMALL}
              />
            </div>
            <div className="step">
              <h5>Step 3:</h5>
              <p>
                Open WireGuard and select &quot;Add Tunnel&quot; or &quot;Import
                tunnel(s) from file&quot;. Find your Defguard configuration file
                and hit &quot;ok&quot;.
              </p>
              <img src={AutoConfigImageOne} alt="auto-config-step-3" />
            </div>
            <div className="step">
              <h5>Step 4:</h5>
              <p>
                Select your tunnel from the list and press &quot;activate&quot;.
              </p>
              <img src={AutoConfigImageTwo} alt="auto-config-step-4" />
            </div>
            <div className="step">
              <h4>Great work - your Defguard VPN is now active! </h4>
              <p></p>
            </div>
          </>
        ) : (
          <>
            <div className="step">
              <h5>Step 1:</h5>
              <p>
                Add a new device in your account settings using the &quot;manual
                configuration&quot; setup and your WireGuard public key.
              </p>
            </div>
            <div className="step">
              <h5>Step 2:</h5>
              <p>Download and install WireGuard client.</p>
              <Button
                className="small"
                size={ButtonSize.SMALL}
                text="Download Wireguard"
              />
            </div>
            <div className="step">
              <h5>Step 3:</h5>
              <p>
                Open WireGuard and select &quot;Add Tunnel&quot;, then &quot;Add
                empty tunnel…&quot;.
              </p>
              <img src={ManualConfigImageOne} alt="manual-config-step3" />
            </div>
            <div className="step">
              <h5>Step 4:</h5>
              <p>
                Type your tunnel name, then copy the configuration below to you
                tunnel settings. Input your private and public keys and click
                &quot;save&quot;.
              </p>
              <MessageBox
                type={MessageBoxType.ERROR}
                message="Remember: Never give your private keys to anyone and keep them secure at all times!"
              />
              <div className="wireguard-config">
                <p>
                  <span className="scope">[Interface]</span>
                </p>
                <p>
                  <span className="var">PrivateKey</span> = YOUR_PRIVATE_KEY=
                </p>
                <p>
                  <span className="var">Address</span> ={' '}
                  <span className="ip">10.6.0.19</span>/32
                </p>
                <p>
                  <span className="var">DNS</span> ={' '}
                  <span className="ip">10.4.0.1</span>,
                  <span className="ip">10.4.0.2</span>
                </p>
                <br />
                <p>
                  <span className="scope">[Peer]</span>
                </p>
                <p>
                  <span className="var">PublicKey</span> = YOUR_PUBLIC_KEY=
                </p>
                <p>
                  <span className="var">AllowedIPs</span> ={' '}
                  <span className="ip">10.4.0.0</span>/24,{' '}
                  <span className="ip">10.50.0.0</span>/24,{' '}
                  <span className="ip">10.7.0.0</span>/16
                </p>
                <p>
                  <span className="var">Endpoint</span> = vpn.teonite.net:7301
                </p>
                <p>
                  <span className="var">PersistentKeepalive</span> = 25
                </p>
              </div>
              <Button
                className="copy"
                size={ButtonSize.SMALL}
                text={!copied ? 'Copy configuration' : 'Copied!'}
                onClick={() => handleCopy()}
              />
            </div>
            <div className="step">
              <p>
                Now just click &quot;activate&quot; to use your
                DefGuard/WireGuard VPN tunnel.
              </p>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
};

export default UserEditVpnHelpModal;
