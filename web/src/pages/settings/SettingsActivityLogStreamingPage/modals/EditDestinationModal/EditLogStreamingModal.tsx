import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import {
  type ActivityLogStream,
  ActivityLogStreamType,
  type CreateActivityLogStreamRequest,
} from '../../../../../shared/api/types';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { processCertificateFile } from '../../../../../shared/utils/processCertificateFile';

const modalNameValue = ModalName.EditLogStreaming;

type ModalData = ActivityLogStream;

export const EditLogStreamingModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  const modalTitle = useMemo(() => {
    if (!modalData) return m.modal_edit_log_streaming_destination_title();
    switch (modalData.stream_type) {
      case ActivityLogStreamType.LogstashHttp:
        return m.modal_edit_logstash_destination_title();
      case ActivityLogStreamType.VectorHttp:
        return m.modal_edit_vector_destination_title();
    }
  }, [modalData]);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data: ModalData) => {
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
      {isPresent(modalData) && (
        <ModalContent key={modalData.id} modalData={modalData} setOpen={setOpen} />
      )}
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
        username: z.string().nullable(),
        password: z.string().nullable(),
        certificate: z.file().nullable(),
      }),
    [],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      name: modalData.name,
      url: modalData.config.url,
      username: modalData.config.username,
      password: modalData.config.password,
      certificate: modalData.config.cert
        ? new File([modalData.config.cert], 'certificate.pem')
        : null,
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
      const certificateContent = await processCertificateFile(value.certificate);

      const requestData: CreateActivityLogStreamRequest = {
        name: value.name,
        stream_type: modalData.stream_type,
        stream_config: {
          url: value.url,
          username: value.username,
          password: value.password,
          cert: certificateContent,
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
            {(field) => <field.FormUploadField title="Upload certificate file" />}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        submitProps={{
          text: m.controls_save_changes(),
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
