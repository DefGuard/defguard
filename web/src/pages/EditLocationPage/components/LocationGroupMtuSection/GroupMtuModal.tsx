import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { SelectionSection } from '../../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionOption } from '../../../../shared/components/SelectionSection/type';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { FieldError } from '../../../../shared/defguard-ui/components/FieldError/FieldError';
import { Input } from '../../../../shared/defguard-ui/components/Input/Input';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { type GroupClientMtu, maxMtu, minMtu } from './types';

type Props = {
  isOpen: boolean;
  target?: Partial<GroupClientMtu>;
  /** Groups not assigned to any other override. */
  groupOptions: SelectionOption<number>[];
  onClose: () => void;
  afterClose: () => void;
  onSubmit: (value: GroupClientMtu) => void;
};

export const GroupMtuModal = ({
  isOpen,
  target,
  groupOptions,
  onClose,
  afterClose,
  onSubmit,
}: Props) => {
  const [onGroupStep, setOnGroupStep] = useState(false);

  return (
    <Modal
      title={
        onGroupStep
          ? m.location_network_group_mtu_groups_modal_title()
          : m.location_network_group_mtu_value_modal_title()
      }
      id="group-mtu-modal"
      contentClassName="group-mtu-modal"
      isOpen={isOpen}
      onClose={onClose}
      afterClose={() => {
        setOnGroupStep(false);
        afterClose();
      }}
    >
      {isPresent(target) && (
        <GroupMtuModalContent
          target={target}
          groupOptions={groupOptions}
          onGroupStep={onGroupStep}
          setOnGroupStep={setOnGroupStep}
          onClose={onClose}
          onSubmit={onSubmit}
        />
      )}
    </Modal>
  );
};

type ContentProps = {
  target: Partial<GroupClientMtu>;
  groupOptions: SelectionOption<number>[];
  onGroupStep: boolean;
  setOnGroupStep: (value: boolean) => void;
  onClose: () => void;
  onSubmit: (value: GroupClientMtu) => void;
};

const GroupMtuModalContent = ({
  target,
  groupOptions,
  onGroupStep,
  setOnGroupStep,
  onClose,
  onSubmit,
}: ContentProps) => {
  const [clientMtu, setClientMtu] = useState<number | null>(target.client_mtu ?? null);
  const [mtuError, setMtuError] = useState<string>();
  const [groupIds, setGroupIds] = useState(new Set(target.group_ids ?? []));
  const [groupError, setGroupError] = useState<string>();

  const validateMtu = (value: number | null): string | undefined => {
    if (value === null) return m.form_error_required();
    if (value < minMtu) return m.form_error_min({ value: minMtu });
    if (value > maxMtu) return m.form_error_invalid();
  };

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
