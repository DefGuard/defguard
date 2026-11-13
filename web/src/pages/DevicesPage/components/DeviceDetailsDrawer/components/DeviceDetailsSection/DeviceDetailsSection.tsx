import './style.scss';
import {
  Icon,
  type IconKindValue,
} from '../../../../../../shared/defguard-ui/components/Icon';
import type { DeviceDetailsRow } from './types';

type Props = {
  icon: IconKindValue;
  title: string;
  rows: DeviceDetailsRow[];
};

export const DeviceDetailsSection = ({ icon, title, rows }: Props) => {
  return (
    <div className="device-details-section">
      <div className="header">
        <Icon icon={icon} />
        <p className="title">{title}</p>
      </div>
      <div className="rows">
        {rows.map((row) => (
          <div className="row" key={row.label}>
            <span className="label">{row.label}</span>
            <div className="value">{row.value}</div>
          </div>
        ))}
      </div>
    </div>
  );
};
