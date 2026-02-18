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
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import amazonImage from '../assets/amazon.png';
import kubernetesImage from '../assets/kub.png';
import teraImage from '../assets/terra.png';
import { EdgeSetupStep } from '../types';
import { useEdgeWizardStore } from '../useEdgeWizardStore';

type TabItem = 'docker' | 'compose' | 'package' | 'virtualImage' | 'other';

export const SetupEdgeDeployStep = () => {
  const [confirmed, setConfirmed] = useState(false);
  const [activeTab, setActiveTab] = useState<TabItem>('docker');

  const tabsConfig = useMemo(
    (): TabsItem[] => [
      {
        title: m.edge_setup_step_deploy_tabs_docker(),
        onClick: () => setActiveTab('docker'),
        active: activeTab === 'docker',
        hidden: false,
      },
      {
        title: m.edge_setup_step_deploy_tabs_compose(),
        onClick: () => setActiveTab('compose'),
        active: activeTab === 'compose',
        hidden: false,
      },
      {
        title: m.edge_setup_step_deploy_tabs_package(),
        onClick: () => setActiveTab('package'),
        active: activeTab === 'package',
        hidden: false,
      },
      {
        title: m.edge_setup_step_deploy_tabs_virtual_image(),
        onClick: () => setActiveTab('virtualImage'),
        active: activeTab === 'virtualImage',
        hidden: false,
      },
      {
        title: m.edge_setup_step_deploy_tabs_other(),
        onClick: () => setActiveTab('other'),
        active: activeTab === 'other',
        hidden: false,
      },
    ],
    [activeTab],
  );
  return (
    <WizardCard id="edge-deploy-step">
      <Tabs items={tabsConfig} disablePadding />
      <SizedBox height={ThemeSpacing.Xl2} />
      {tabsContent[activeTab]}
      <SizedBox height={ThemeSpacing.Xl2} />
      <Checkbox
        active={confirmed}
        onClick={() => {
          setConfirmed((s) => !s);
        }}
        text={m.edge_setup_step_deploy_confirm()}
      />
      <SizedBox height={ThemeSpacing.Xl3} />
      <Controls>
        <Button
          variant={'outlined'}
          text={m.controls_back()}
          onClick={() => {
            useEdgeWizardStore.setState({
              isOnWelcomePage: true,
            });
          }}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            disabled={!confirmed}
            onClick={() => {
              useEdgeWizardStore.setState({
                activeStep: EdgeSetupStep.EdgeComponent,
              });
            }}
          />
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
        title={m.edge_setup_step_deploy_tabs_compose_title()}
        subtitle={m.edge_setup_step_deploy_tabs_compose_subtitle({
          filename: `docker-compose.yaml`,
        })}
      />
      <CodeSnippet
        value={`services:
  proxy:
    image: ghcr.io/defguard/defguard-proxy:latest
    restart: unless-stopped
    ports:
      - "127.0.0.1:8080:8080"
      - "50051:50051"`}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <AppText font={TextStyle.TBodySm400}>
        {m.edge_setup_step_deploy_tabs_compose_then()}
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
        title={m.edge_setup_step_deploy_tabs_docker_title()}
        subtitle={m.edge_setup_step_deploy_tabs_docker_subtitle()}
      />
      <CodeSnippet
        value={`docker run --restart unless-stopped ghcr.io/defguard/defguard-proxy:latest`}
      />
    </>
  );
};

const PackageTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.edge_setup_step_deploy_tabs_package_title()}
        subtitle={m.edge_setup_step_deploy_tabs_package_subtitle()}
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
sudo apt install defguard-proxy`}
      />
    </>
  );
};

const VirtualImageTab = () => {
  return (
    <>
      <TabContentHeader
        title={m.edge_setup_step_deploy_tabs_virtual_title()}
        subtitle={m.edge_setup_step_deploy_tabs_virtual_subtitle({
          url: `https://defguard.net/download/defguard-2x-latest.ovf`,
          filename: `cloud-init-yaml`,
        })}
      />
      <CodeSnippet
        value={`launch: gateway

runcmd:
  - |
    LAUNCH=$(jq -r '.["user-data"].launch' /run/cloud-init/instance-data.json)
    
    echo "Launch option = $LAUNCH"
    
    if [ "$LAUNCH" = "gateway" ]; then
      systemctl enable defguard-gateway
      systemctl start defguard-gateway
    else
      echo "Unknown launch: $LAUNCH"
    fi`}
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
        title={m.edge_setup_step_deploy_tabs_other_title()}
        subtitle={m.edge_setup_step_deploy_tabs_other_subtitle()}
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
        {m.edge_setup_step_deploy_tabs_other_launch()}
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
