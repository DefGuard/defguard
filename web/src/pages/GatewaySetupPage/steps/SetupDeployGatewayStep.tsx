import { type ReactNode, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Card } from '../../../shared/components/Card/Card';
import { CodeSnippet } from '../../../shared/components/CodeSnippet/CodeSnippet';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Icon, IconKind } from '../../../shared/defguard-ui/components/Icon';
import { RenderMarkdown } from '../../../shared/defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../../shared/defguard-ui/components/Tabs/types';
import { TooltipContent } from '../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import amazonImage from '../assets/amazon.png';
import kubernetesImage from '../assets/kub.png';
import teraImage from '../assets/terra.png';
import { GatewaySetupStep } from '../types';
import { useGatewayWizardStore } from '../useGatewayWizardStore';

type TabItem = 'docker' | 'compose' | 'package' | 'virtualImage' | 'other';

export const SetupDeployGatewayStep = () => {
  const [confirmed, setConfirmed] = useState(false);
  const [activeTab, setActiveTab] = useState<TabItem>('docker');

  const tabsConfig = useMemo(
    (): TabsItem[] => [
      {
        title: m.gateway_setup_step_deploy_tabs_docker(),
        onClick: () => setActiveTab('docker'),
        active: activeTab === 'docker',
        hidden: false,
      },
      {
        title: m.gateway_setup_step_deploy_tabs_compose(),
        onClick: () => setActiveTab('compose'),
        active: activeTab === 'compose',
        hidden: false,
      },
      {
        title: m.gateway_setup_step_deploy_tabs_package(),
        onClick: () => setActiveTab('package'),
        active: activeTab === 'package',
        hidden: false,
      },
      {
        title: m.gateway_setup_step_deploy_tabs_virtual_image(),
        onClick: () => setActiveTab('virtualImage'),
        active: activeTab === 'virtualImage',
        hidden: false,
      },
      {
        title: m.gateway_setup_step_deploy_tabs_other(),
        onClick: () => setActiveTab('other'),
        active: activeTab === 'other',
        hidden: false,
      },
    ],
    [activeTab],
  );
  return (
    <WizardCard id="deploy-gateway-step">
      <Tabs items={tabsConfig} disablePadding />
      <SizedBox height={ThemeSpacing.Xl2} />
      {tabsContent[activeTab]}
      <SizedBox height={ThemeSpacing.Xl2} />
      <Checkbox
        active={confirmed}
        onClick={() => {
          setConfirmed((s) => !s);
        }}
        text={m.gateway_setup_step_deploy_confirm()}
      />
      <Controls>
        <Button
          variant={'outlined'}
          text={m.controls_back()}
          onClick={() => {
            useGatewayWizardStore.setState({
              isOnWelcomePage: true,
            });
          }}
        />
        <div className="right">
          <TooltipProvider disabled={confirmed}>
            <TooltipTrigger>
              <div>
                <Button
                  text={m.controls_continue()}
                  disabled={!confirmed}
                  onClick={() => {
                    useGatewayWizardStore.setState({
                      activeStep: GatewaySetupStep.GatewayComponent,
                    });
                  }}
                />
              </div>
            </TooltipTrigger>
            <TooltipContent>
              <p>{m.gateway_setup_step_deploy_confirm_tooltip()}</p>
            </TooltipContent>
          </TooltipProvider>
        </div>
      </Controls>
    </WizardCard>
  );
};

const TabContentHeader = ({ subtitle, title }: { title: string; subtitle: string }) => {
  return (
    <div className="tab-content-header">
      <AppText font={TextStyle.TBodyPrimary500}>{title}</AppText>
      <SizedBox height={ThemeSpacing.Sm} />
      <RenderMarkdown content={subtitle} />
      <SizedBox height={ThemeSpacing.Md} />
    </div>
  );
};

const DockerComposeTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.gateway_setup_step_deploy_tabs_compose_title()}
        subtitle={m.gateway_setup_step_deploy_tabs_compose_subtitle({
          filename: `docker-compose.yaml`,
        })}
      />
      <CodeSnippet
        value={`services:
  gateway:
    image: ghcr.io/defguard/gateway:latest
    restart: unless-stopped
    network_mode: host
    cap_add:
      - NET_ADMIN
    # If you prefer only one port:
    #ports:
    #  - "50066:50066"
    # Uncomment the following if you are running on Debian 13 or later or have apparmor or SELinux setup
    #security_opt:
    #  - apparmor:unconfined
    volumes:
      - ./.volumes/certs/gateway:/etc/defguard/certs`}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <AppText font={TextStyle.TBodySm400}>
        {m.gateway_setup_step_deploy_tabs_compose_then()}
      </AppText>
      <SizedBox height={ThemeSpacing.Md} />
      <CodeSnippet value={`docker compose up -d`} />
    </>
  );
};

const DockerTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.gateway_setup_step_deploy_tabs_docker_title()}
        subtitle={m.gateway_setup_step_deploy_tabs_docker_subtitle()}
      />
      <CodeSnippet
        value={`docker run -v ./.volumes/certs/gateway:/etc/defguard/certs --restart unless-stopped --security-opt apparmor:unconfined --network host --cap-add NET_ADMIN ghcr.io/defguard/gateway:latest`}
      />
    </>
  );
};

const PackageTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.gateway_setup_step_deploy_tabs_package_title()}
        subtitle={m.gateway_setup_step_deploy_tabs_package_subtitle()}
      />
      <CodeSnippet
        value={`sudo apt update 
sudo apt install -y ca-certificates curl 
#Add official Defguard public GPG key
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://apt.defguard.net/defguard.asc -o /etc/apt/keyrings/defguard.asc
sudo chmod a+r /etc/apt/keyrings/defguard.asc

#Add APT repository
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/defguard.asc] https://apt.defguard.net/ trixie release " | \
   sudo tee /etc/apt/sources.list.d/defguard.list > /dev/null 

sudo apt update
sudo apt install defguard-gateway`}
      />
    </>
  );
};

const VirtualImageTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.gateway_setup_step_deploy_tabs_virtual_title()}
        subtitle={m.gateway_setup_step_deploy_tabs_virtual_subtitle({
          url: `https://defguard-downloads.s3.eu-central-1.amazonaws.com/defguard-alpha2.ova`,
          filename: `defguard-data.yaml`,
        })}
      />
      <CodeSnippet
        value={`#cloud-config
write_files:
  - path: /opt/defguard/active-profiles
    permissions: '0644'
    content: |
      gateway
`}
      />
    </>
  );
};

const OtherDeploymentMethod = ({
  image,
  link,
  name,
}: {
  image: string;
  name: string;
  link: string;
}) => {
  return (
    <Card>
      <div className="inner-track">
        <div className="image-tack">
          <img src={image} width={44} height={44} />
        </div>
        <div className="content">
          <p className="title">{name}</p>
          <a target="_blank" rel="noopener noreferrer" href={link}>
            <span>{link}</span>
            <Icon icon={IconKind.OpenInNewWindow} size={16} />
          </a>
        </div>
      </div>
    </Card>
  );
};

const OthersTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.gateway_setup_step_deploy_tabs_other_title()}
        subtitle={m.gateway_setup_step_deploy_tabs_other_subtitle()}
      />
      <div id="other-deployment-methods">
        <OtherDeploymentMethod
          name={`Kubernetes`}
          link={`https://docs.defguard.net/deployment-strategies/kubernetes#deployment`}
          image={kubernetesImage}
        />
        <OtherDeploymentMethod
          name={`Amazon Machine Image`}
          link={`https://docs.defguard.net/deployment-strategies/amis-and-aws-cloudformation`}
          image={amazonImage}
        />
        <OtherDeploymentMethod
          name={`Terraform`}
          link={`https://docs.defguard.net/deployment-strategies/terraform`}
          image={teraImage}
        />
      </div>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
        {m.gateway_setup_step_deploy_tabs_other_launch()}
      </AppText>
    </>
  );
};

const tabsContent: Record<TabItem, ReactNode> = {
  docker: <DockerTab />,
  compose: <DockerComposeTab />,
  package: <PackageTab />,
  virtualImage: <VirtualImageTab />,
  other: <OthersTab />,
};
