import './style.scss';

import { useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import type { SelectionOption } from '../../../../shared/components/SelectionSection/type';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { GroupMtuCard } from './GroupMtuCard';
import { GroupMtuModal } from './GroupMtuModal';
import type { GroupClientMtu } from './types';

type Props = {
  overrides: GroupClientMtu[];
  groupOptions: SelectionOption<number>[];
  onChange: (overrides: GroupClientMtu[]) => void;
};

/** An editor without an `index` creates a new override. */
type OpenEditor = { index?: number };

export const LocationGroupMtuSection = ({ overrides, groupOptions, onChange }: Props) => {
  const [editor, setEditor] = useState<OpenEditor>();
  const [modalOpen, setModalOpen] = useState(false);

  const openEditor = (next: OpenEditor) => {
    setEditor(next);
    setModalOpen(true);
  };

  const groupNameById = useMemo(
    () => new Map(groupOptions.map((option) => [option.id, option.label])),
    [groupOptions],
  );

  const editorTarget = useMemo(() => {
    if (editor === undefined) return undefined;
    return editor.index === undefined ? {} : overrides[editor.index];
  }, [editor, overrides]);

  // A group can have only one MTU override, so hide groups taken by other overrides.
  const editorGroupOptions = useMemo(() => {
    const taken = new Set(
      overrides
        .filter((_, index) => index !== editor?.index)
        .flatMap((override) => override.group_ids),
    );
    return groupOptions.filter((option) => !taken.has(option.id));
  }, [overrides, groupOptions, editor]);

  const allGroupsAssigned =
    groupOptions.length > 0 &&
    groupOptions.every((option) =>
      overrides.some((override) => override.group_ids.includes(option.id)),
    );

  const handleSubmit = (value: GroupClientMtu) => {
    if (editor?.index === undefined) {
      onChange([...overrides, value]);
    } else {
      onChange(
        overrides.map((override, index) => (index === editor.index ? value : override)),
      );
    }
  };

  return (
    <div className="location-group-mtu">
      <p className="title">{m.location_network_group_mtu_title()}</p>
      <p className="description">{m.location_network_group_mtu_description()}</p>
      {overrides.length > 0 && (
        <div className="overrides">
          {overrides.map((override, index) => (
            <GroupMtuCard
              key={override.group_ids.join(',')}
              clientMtu={override.client_mtu}
              chips={override.group_ids.map((id) => groupNameById.get(id) ?? String(id))}
              onEdit={() => openEditor({ index })}
              onRemove={() => onChange(overrides.filter((_, other) => other !== index))}
            />
          ))}
        </div>
      )}
      <Button
        variant="outlined"
        text={m.location_network_group_mtu_add()}
        disabled={allGroupsAssigned}
        onClick={() => openEditor({})}
      />
      <GroupMtuModal
        isOpen={modalOpen}
        target={editorTarget}
        groupOptions={editorGroupOptions}
        onClose={() => setModalOpen(false)}
        afterClose={() => setEditor(undefined)}
        onSubmit={handleSubmit}
      />
    </div>
  );
};
