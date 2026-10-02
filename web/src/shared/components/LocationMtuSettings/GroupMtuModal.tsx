import { useState } from 'react';
import { m } from '../../../paraglide/messages';
import type { GroupClientMtu } from '../../api/types';
import { MAX_MTU, MIN_MTU } from '../../constants';
import { Button } from '../../defguard-ui/components/Button/Button';
import { FieldError } from '../../defguard-ui/components/FieldError/FieldError';
import { Input } from '../../defguard-ui/components/Input/Input';
import { Modal } from '../../defguard-ui/components/Modal/Modal';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { Controls } from '../Controls/Controls';
import { SelectionSection } from '../SelectionSection/SelectionSection';
import type { SelectionOption } from '../SelectionSection/type';

type Props = {
  isOpen: boolean;
  /** Mounts the content while defined; `null` creates a new override. */
  initial?: GroupClientMtu | null;
  /** Groups not assigned to any other override. */
  groupOptions: SelectionOption<number>[];
  onClose: () => void;
  afterClose: () => void;
  onSubmit: (value: GroupClientMtu) => void;
};

export const GroupMtuModal = ({
  isOpen,
  initial,
  groupOptions,
  onClose,
  afterClose,
  onSubmit,
}: Props) => (
  <Modal
    title={m.location_network_group_mtu_modal_title()}
    id="group-mtu-modal"
    contentClassName="group-mtu-modal"
    isOpen={isOpen}
    onClose={onClose}
    afterClose={afterClose}
  >
    {initial !== undefined && (
      <GroupMtuModalContent
        initial={initial}
        groupOptions={groupOptions}
        onClose={onClose}
        onSubmit={onSubmit}
      />
    )}
  </Modal>
);

type ContentProps = {
  initial: GroupClientMtu | null;
  groupOptions: SelectionOption<number>[];
  onClose: () => void;
  onSubmit: (value: GroupClientMtu) => void;
};

const validateMtu = (value: number | null): string | undefined => {
  if (value === null) return m.form_error_required();
  if (value < MIN_MTU) return m.form_error_min({ value: MIN_MTU });
  if (value > MAX_MTU) return m.form_error_invalid();
};

const GroupMtuModalContent = ({
  initial,
  groupOptions,
  onClose,
  onSubmit,
}: ContentProps) => {
  const [onGroupStep, setOnGroupStep] = useState(false);
  const [clientMtu, setClientMtu] = useState(initial?.client_mtu ?? null);
  const [mtuError, setMtuError] = useState<string>();
  const [groupIds, setGroupIds] = useState(new Set(initial?.group_ids));
  const [groupError, setGroupError] = useState<string>();

  const handleMtuChange = (value: string | number | null) => {
    const next = typeof value === 'number' ? value : null;
    setClientMtu(next);
    if (isPresent(mtuError)) setMtuError(validateMtu(next));
  };

  const handleContinue = () => {
    const error = validateMtu(clientMtu);
    setMtuError(error);
    if (error === undefined) setOnGroupStep(true);
  };

  const handleGroupChange = (next: Set<number>) => {
    setGroupIds(next);
    if (next.size > 0) setGroupError(undefined);
  };

  const handleSubmit = () => {
    if (clientMtu === null) return;
    if (groupIds.size === 0) {
      setGroupError(m.location_network_group_mtu_groups_required());
      return;
    }
    onSubmit({
      client_mtu: clientMtu,
      group_ids: [...groupIds].sort((left, right) => left - right),
    });
    onClose();
  };

  return (
    <>
      {onGroupStep ? (
        <>
          <SelectionSection
            options={groupOptions}
            selection={groupIds}
            onChange={handleGroupChange}
            searchPlaceholder={m.controls_search()}
            visibleItemsLimit={10}
          />
          <FieldError error={groupError} />
        </>
      ) : (
        <>
          <p className="description">
            {m.location_network_group_mtu_value_description()}
          </p>
          <SizedBox height={ThemeSpacing.Xl} />
          <Input
            required
            type="number"
            label={m.location_network_group_mtu_value_label()}
            value={clientMtu}
            error={mtuError}
            onChange={handleMtuChange}
          />
        </>
      )}
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        {onGroupStep && (
          <Button
            variant="outlined"
            text={m.controls_back()}
            onClick={() => setOnGroupStep(false)}
          />
        )}
        <div className="right">
          <Button variant="secondary" text={m.controls_cancel()} onClick={onClose} />
          {onGroupStep ? (
            <Button text={m.controls_submit()} onClick={handleSubmit} />
          ) : (
            <Button text={m.controls_continue()} onClick={handleContinue} />
          )}
        </div>
      </Controls>
    </>
  );
};
