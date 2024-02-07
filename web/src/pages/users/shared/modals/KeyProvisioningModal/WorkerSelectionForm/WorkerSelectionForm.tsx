import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import { Controller, SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { Provisioner } from '../../../../../../shared/types';
import WorkerSelectItem from './WorkerSelectItem';

interface Props {
  setIsOpen: (v: boolean) => void;
  afterSubmit: (v: number) => void;
  workers?: Provisioner[];
}

interface FormValues {
  worker: string;
}

export const WorkerSelectionForm = ({ setIsOpen, afterSubmit, workers }: Props) => {
  const { LL } = useI18nContext();
  const {
    provisioning: { provisionYubiKey },
  } = useApi();

  const username = useModalStore((state) => state.provisionKeyModal.user?.username);

  const { mutate: createJob } = useMutation(provisionYubiKey, {
    onSuccess: (responseData) => {
      afterSubmit(responseData.id);
    },
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        worker: z.string().min(1, LL.form.error.required()),
      }),
    [LL.form.error],
  );

  const {
    control,
    handleSubmit,
    formState: { isValid },
    resetField,
  } = useForm<FormValues>({
    resolver: zodResolver(zodSchema),
    mode: 'all',
  });

  const onSubmit: SubmitHandler<FormValues> = (data) => {
    if (username) {
      createJob({
        worker: data.worker,
        username: username,
      });
    }
  };

  if (!workers) return null;

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <label>{LL.modals.provisionKeys.selectionLabel()}</label>
      <ul>
        {workers.map((worker) => (
          <li key={worker.id}>
            <Controller
              control={control}
              name="worker"
              render={({ field: { onChange, value, ref } }) => (
                <WorkerSelectItem
                  ref={ref}
                  value={value}
                  onChange={(val: string) => {
                    if (val === value) {
                      resetField('worker');
                    } else {
                      onChange(val);
                    }
                  }}
                  expected={worker.id}
                  active={worker.connected}
                />
              )}
            />
          </li>
        ))}
      </ul>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.LARGE}
          text={LL.form.cancel()}
          className="close"
          onClick={() => {
            resetField('worker');
            setIsOpen(false);
          }}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          disabled={!isValid}
          text={LL.modals.provisionKeys.controls.submit()}
          type="submit"
        />
      </div>
    </form>
  );
};
