import { BrandingCard } from '../BrandingCard/BrandingCard';
import { BuiltByCard } from '../BuiltByCard/BuiltByCard';
import { ModulesCard } from '../ModulesCard/ModulesCard';
import { Web3Settings } from '../Web3Settings/Web3Settings';

export const GeneralTab = () => (
  <>
    <div className="left">
      <BrandingCard />
      <ModulesCard />
      {/*<DefaultNetworkSelect /> */}
    </div>
    <div className="right">
      <Web3Settings />
      <BuiltByCard />
    </div>
  </>
);
