import { DebugDataCard } from '../DebugDataCard/DebugDataCard';
import { SupportCard } from '../SupportCard/SupportCard';

export const SupportTab = () => (
  <>
    <div className="left">
      <DebugDataCard />
    </div>
    <div className="right">
      <SupportCard />
    </div>
  </>
);
