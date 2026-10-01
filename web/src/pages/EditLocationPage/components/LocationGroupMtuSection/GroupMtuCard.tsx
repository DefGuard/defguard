import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Chip } from '../../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { Icon } from '../../../../shared/defguard-ui/components/Icon/Icon';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';

const collapsedChipLimit = 5;

type Props = {
  clientMtu: number;
  chips: string[];
  onEdit: () => void;
  onRemove: () => void;
};

export const GroupMtuCard = ({ clientMtu, chips, onEdit, onRemove }: Props) => {
  const [expanded, setExpanded] = useState(false);
  const foldable = chips.length > collapsedChipLimit;
  const visibleChips = expanded ? chips : chips.slice(0, collapsedChipLimit);

  return (
    <div className="group-mtu-card">
      <div className="header">
        <p className="mtu">
          <span className="label">{m.location_network_group_mtu_value_label()}</span>{' '}
          <span className="value">{clientMtu}</span>
        </p>
        <div className="actions">
          <button
            type="button"
            className="card-action"
            aria-label={m.location_network_group_mtu_edit()}
            onClick={onEdit}
          >
            <Icon icon={IconKind.Edit} size={20} />
          </button>
          <button
            type="button"
            className="card-action"
            aria-label={m.location_network_group_mtu_remove()}
            onClick={onRemove}
          >
            <Icon icon={IconKind.Delete} size={20} />
          </button>
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Md} />
      <div className="chips">
        {visibleChips.map((chip) => (
          <Chip text={chip} key={chip} />
        ))}
      </div>
      {foldable && (
        <button
          type="button"
          className="fold-chips"
          onClick={() => setExpanded((value) => !value)}
        >
          {expanded ? m.controls_show_less() : m.controls_show_more()}
        </button>
      )}
    </div>
  );
};
