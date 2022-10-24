import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation } from '@tanstack/react-query';
import React from 'react';
import { Controller, SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
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

const WorkerSelectionForm: React.FC<Props> = ({
  setIsOpen,
  afterSubmit,
  workers,
}) => {
  const {
    provisioning: { provisionYubiKey },
  } = useApi();

  const username = useModalStore(
    (state) => state.provisionKeyModal.user?.username
  );

  const { mutate: createJob } = useMutation(provisionYubiKey, {
    onSuccess: (responseData) => {
      afterSubmit(responseData.id);
    },
  });

  const formSchema = yup.object({
    worker: yup.string().required(),
  });

  const {
    control,
    handleSubmit,
    formState: { isValid },
    resetField,
  } = useForm<FormValues>({
    resolver: yupResolver(formSchema),
    mode: 'onChange',
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
      <label>
        Select one of the following provisioners to provision a YubiKey:
      </label>
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
          size={ButtonSize.BIG}
          text="Cancel"
          className="close"
          onClick={() => {
            resetField('worker');
            setIsOpen(false);
          }}
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          disabled={!isValid}
          text="Provision YubiKey"
          type="submit"
        />
      </div>
    </form>
  );
};

export default WorkerSelectionForm;
