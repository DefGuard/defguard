import { zodResolver } from '@hookform/resolvers/zod';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { FormDateInput } from '../../../shared/components/Layout/DateInput/FormDateInput';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';

type Props = {
  isOpen: boolean;
  onOpenChange: (val: boolean) => void;
  // Time in UTC ISO without timezone string
  activityFrom: string | null;
  activityUntil: string | null;
  onChange: (from: string | null, until: string | null) => void;
};

export const ActivityTimeRangeModal = (props: Props) => {
  return (
    <ModalWithTitle
      title="Activity time"
      isOpen={props.isOpen}
      onClose={() => {
        props.onOpenChange(false);
      }}
    >
      <ModalContent {...props} />
    </ModalWithTitle>
  );
};

const ModalContent = ({ onOpenChange, activityFrom, activityUntil, onChange }: Props) => {
  const schema = useMemo(
    () =>
      z.object({
        from: z.string().nullable(),
        until: z.string().nullable(),
      }),
    [],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      from: activityFrom,
      until: activityUntil,
    }),
    [activityFrom, activityUntil],
  );

  const { control, handleSubmit } = useForm({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    onChange(values.from, values.until);
    onOpenChange(false);
  };

  return (
    <>
      <form
        onSubmit={handleSubmit(handleValidSubmit)}
        id="activity-time-selection-modal-form"
      >
        <FormDateInput
          clearable
          label="From"
          controller={{ control, name: 'from' }}
          showTimeSelection
        />
        <FormDateInput
          clearable
          label="Until"
          controller={{ control, name: 'until' }}
          showTimeSelection
        />
        <div className="controls">
          <Button
            type="button"
            size={ButtonSize.STANDARD}
            text="Cancel"
            onSubmit={() => {
              onOpenChange(false);
            }}
          />
          <Button
            type="submit"
            size={ButtonSize.STANDARD}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text="Save"
          />
        </div>
      </form>
    </>
  );
};
