import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { useEffect, useMemo } from 'react';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { SectionSelect } from '../../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { useAddDestinationModal } from './useAddDestinationModal';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { m } from '../../../../../paraglide/messages';
import { useStore } from '@tanstack/react-form';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import z from 'zod';
import { ModalControls } from '../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import api from '../../../../../shared/api/api';
import {
  ActivityLogStreamType,
  type CreateActivityLogStreamRequest,
} from '../../../../../shared/api/types';

const modalNameValue = ModalName.AddLogStreaming;

export const AddLogStreamingModal = () => {
  const isOpen = useAddDestinationModal((s) => s.isOpen);
  const step = useAddDestinationModal((s) => s.step);
  const destination = useAddDestinationModal((s) => s.destination);

  const modalTitle = useMemo(() => {
    if (step === 'choice') return 'Select destination';
    switch (destination) {
      case 'logstash':
        return 'Add Logstash destination';
      case 'vector':
        return 'Add Vector destination';
    }
  }, [step, destination]);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, () => {
      useAddDestinationModal.setState({ isOpen: true });
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => {
      useAddDestinationModal.setState({ isOpen: false });
    });
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title={modalTitle}
      isOpen={isOpen}
      onClose={() => useAddDestinationModal.setState({ isOpen: false })}
      afterClose={() => useAddDestinationModal.getState().reset()}
    >
      {step === 'choice' && <ChoiceStep />}
      {step === 'form' && <FormStep />}
    </Modal>
  );
};

const ChoiceStep = () => {
  return (
    <>
      <SectionSelect
        image="logstash"
        title="Logstash"
        content={m.modal_add_logstash_destination()}
        data-testid="add-logstash"
        onClick={() => {
          useAddDestinationModal.setState({
            step: 'form',
            destination: 'logstash',
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        image="vector"
        content={m.modal_add_vector_destination()}
        title="Vector"
        data-testid="add-vector"
        onClick={() => {
          useAddDestinationModal.setState({
            step: 'form',
            destination: 'vector',
          });
        }}
      />
    </>
  );
};

const FormStep = () => {
  const destination = useAddDestinationModal((s) => s.destination);

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
      name: '',
      url: '',
      username: '',
      password: '',
      certificate: '',
    }),
    [],
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
        stream_type:
          destination === 'logstash'
            ? ActivityLogStreamType.LogstashHttp
            : ActivityLogStreamType.VectorHttp,
        stream_config: {
          url: value.url,
          username: value.username || undefined,
          password: value.password || undefined,
          cert: value.certificate || undefined,
        },
      };

      await api.activityLogStream.createStream(requestData);
      useAddDestinationModal.setState({ isOpen: false });
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
            {(field) => <field.FormInput data-testid="field-url" required label="URL" />}
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
          text: m.controls_add_destination(),
          testId: 'add-destination-submit',
          onClick: () => form.handleSubmit(),
        }}
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => {
            useAddDestinationModal.setState({ isOpen: false });
          },
        }}
      ></ModalControls>
    </>
  );
};
