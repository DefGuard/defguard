import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import type { GroupClientMtu } from '../../api/types';
import { AppText } from '../../defguard-ui/components/AppText/AppText';
import { Button } from '../../defguard-ui/components/Button/Button';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
import { getGroupsInfoQueryOptions } from '../../query';
import type { SelectionOption } from '../SelectionSection/type';
import { GroupMtuCard } from './GroupMtuCard';
import { GroupMtuModal } from './GroupMtuModal';

type Props = {
  overrides: GroupClientMtu[];
  onChange: (overrides: GroupClientMtu[]) => void;
};

/** Index of the edited override, or `'new'`. */
type Editing = number | 'new';

export const LocationGroupMtuSection = ({ overrides, onChange }: Props) => {
  const [editing, setEditing] = useState<Editing>();
  const [modalOpen, setModalOpen] = useState(false);

  const { data: groupOptions = [] } = useQuery({
    ...getGroupsInfoQueryOptions,
    select: (response) =>
      response.data.map(
        (group): SelectionOption<number> => ({
          id: group.id,
          label: group.name,
        }),
      ),
  });

  const openEditor = (next: Editing) => {
    setEditing(next);
    setModalOpen(true);
  };

  const groupNameById = useMemo(
    () => new Map(groupOptions.map((option) => [option.id, option.label])),
    [groupOptions],
  );

  // A group can have one override only.
  const editorGroupOptions = useMemo(() => {
    const taken = new Set(
      overrides
        .filter((_, index) => index !== editing)
        .flatMap((override) => override.group_ids),
    );
    return groupOptions.filter((option) => !taken.has(option.id));
  }, [overrides, groupOptions, editing]);

  const allGroupsAssigned =
    groupOptions.length > 0 &&
    groupOptions.every((option) =>
      overrides.some((override) => override.group_ids.includes(option.id)),
    );

  const handleSubmit = (value: GroupClientMtu) => {
    if (editing === 'new') {
      onChange([...overrides, value]);
    } else {
      onChange(
        overrides.map((override, index) => (index === editing ? value : override)),
      );
    }
  };

  return (
    <div className="location-group-mtu">
      <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
        {m.location_network_group_mtu_title()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {m.location_network_group_mtu_description()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      {overrides.length > 0 && (
        <div className="overrides">
          {overrides.map((override, index) => (
            <GroupMtuCard
              key={override.group_ids.join(',')}
              clientMtu={override.client_mtu}
              chips={override.group_ids.map((id) => groupNameById.get(id) ?? String(id))}
              onEdit={() => openEditor(index)}
              onRemove={() => onChange(overrides.filter((_, other) => other !== index))}
            />
          ))}
        </div>
      )}
      <Button
        variant="outlined"
        text={m.location_network_group_mtu_add()}
        disabled={allGroupsAssigned}
        onClick={() => openEditor('new')}
      />
      <GroupMtuModal
        isOpen={modalOpen}
        initial={
          editing === undefined
            ? undefined
            : editing === 'new'
              ? null
              : overrides[editing]
        }
        groupOptions={editorGroupOptions}
        onClose={() => setModalOpen(false)}
        afterClose={() => setEditing(undefined)}
        onSubmit={handleSubmit}
      />
    </div>
  );
};
