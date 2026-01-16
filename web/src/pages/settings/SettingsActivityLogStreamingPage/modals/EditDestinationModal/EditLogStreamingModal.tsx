import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import {
  ActivityLogStreamType,
  type ActivityLogStream,
  type CreateActivityLogStreamRequest,
} from '../../../../../shared/api/types';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';

const modalNameValue = ModalName.EditLogStreaming;

type ModalData = ActivityLogStream;

export const EditLogStreamingModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const modalTitle = useMemo(() => {
    if (!modalData) return 'Edit destination';
    switch (modalData.stream_type) {
      case ActivityLogStreamType.LogstashHttp:
        return 'Edit Logstash destination';
      case ActivityLogStreamType.VectorHttp:
        return 'Edit Vector destination';
    }
  }, [modalData]);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title={modalTitle}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => setModalData(null)}
    >
      {isPresent(modalData) && <ModalContent modalData={modalData} setOpen={setOpen} />}
    </Modal>
  );
};

type ModalContentProps = {
  modalData: ModalData;
  setOpen: (open: boolean) => void;
};

const ModalContent = ({ modalData, setOpen }: ModalContentProps) => {
  const { mutateAsync: updateStream } = useMutation({
    mutationFn: ({ id, data }: { id: number; data: CreateActivityLogStreamRequest }) =>
      api.activityLogStream.updateStream(id, data),
    meta: {
      invalidate: ['activity_log_stream'],
    },
  });

  const formSchema = useMemo(
    () =>
      z.object({
        name: z.string().trim().min(1, m.form_error_required()),
        url: z.string().trim().min(1, m.form_error_required()),
        username: z.string().optional(),
        password: z.string().optional(),
        certificate: z.string().optional(),
      }),
    [],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      name: modalData.name,
      url: modalData.config.url,
      username: modalData.config.username || '',
      password: modalData.config.password || '',
      certificate: modalData.config.cert || '',
    }),
    [modalData],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const requestData: CreateActivityLogStreamRequest = {
        name: value.name,
        stream_type: modalData.stream_type,
        stream_config: {
          url: value.url,
          username: value.username || undefined,
          password: value.password || undefined,
          cert: value.certificate || undefined,
        },
      };

      await updateStream({ id: modalData.id, data: requestData });
      setOpen(false);
    },
  });

  return (
    <>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Name" />}
          </form.AppField>

          <SizedBox height={ThemeSpacing.Xl} />

          <form.AppField name="url">
            {(field) => <field.FormInput required label="URL" />}
          </form.AppField>

          <SizedBox height={ThemeSpacing.Xl} />

          <form.AppField name="username">
            {(field) => <field.FormInput label="Username" />}
          </form.AppField>

          <SizedBox height={ThemeSpacing.Xl} />

          <form.AppField name="password">
            {(field) => <field.FormInput label="Password" type="password" />}
          </form.AppField>

          <SizedBox height={ThemeSpacing.Xl} />

          <form.AppField name="certificate">
            {(field) => <field.FormInput label="Certificate" />}
          </form.AppField>
        </form.AppForm>
      </form>
      <SizedBox height={ThemeSpacing.Xl2} />
      <ModalControls
        submitProps={{
          text: m.controls_save(),
          testId: 'edit-destination-submit',
          onClick: () => form.handleSubmit(),
        }}
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => setOpen(false),
        }}
      />
    </>
  );
};
