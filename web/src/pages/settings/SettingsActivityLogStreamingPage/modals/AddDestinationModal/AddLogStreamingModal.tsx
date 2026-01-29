import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import {
  ActivityLogStreamType,
  type CreateActivityLogStreamRequest,
} from '../../../../../shared/api/types';
import { Modal } from '../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SectionSelect } from '../../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';

const modalNameValue = ModalName.AddLogStreaming;

type ModalState = {
  step: 'choice' | 'form';
  destination: 'logstash' | 'vector' | null;
};

const defaultState: ModalState = {
  step: 'choice',
  destination: null,
};

export const AddLogStreamingModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalState, setModalState] = useState<ModalState>(defaultState);

  const modalTitle = useMemo(() => {
    if (modalState.step === 'choice')
      return m.modal_select_log_streaming_destination_title();
    switch (modalState.destination) {
      case 'logstash':
        return m.modal_add_logstash_destination_title();
      case 'vector':
        return m.modal_add_vector_destination_title();
      default:
        return m.modal_add_log_streaming_destination_title();
    }
  }, [modalState]);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, () => {
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
      afterClose={() => setModalState(defaultState)}
    >
      {modalState.step === 'choice' && <ChoiceStep setModalState={setModalState} />}
      {modalState.step === 'form' && modalState.destination && (
        <FormStep
          destination={modalState.destination}
          setOpen={setOpen}
          setModalState={setModalState}
        />
      )}
    </Modal>
  );
};

type ChoiceStepProps = {
  setModalState: (state: ModalState) => void;
};

const ChoiceStep = ({ setModalState }: ChoiceStepProps) => {
  return (
    <>
      <SectionSelect
        image="logstash"
        title={m.modal_logstash_destination_title()}
        content={m.modal_add_logstash_destination()}
        data-testid="add-logstash"
        onClick={() => {
          setModalState({
            step: 'form',
            destination: 'logstash',
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        image="vector"
        content={m.modal_add_vector_destination()}
        title={m.modal_vector_destination_title()}
        data-testid="add-vector"
        onClick={() => {
          setModalState({
            step: 'form',
            destination: 'vector',
          });
        }}
      />
    </>
  );
};

type FormStepProps = {
  destination: 'logstash' | 'vector';
  setOpen: (open: boolean) => void;
  setModalState: (state: ModalState) => void;
};

const FormStep = ({ destination, setOpen }: FormStepProps) => {
  const { mutateAsync: createStream } = useMutation({
    mutationFn: api.activityLogStream.createStream,
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
        certificate: z.file().nullable().optional(),
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
      certificate: null,
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
      let certificateContent: string | undefined = undefined;
      
      if (value.certificate) {
        try {
          certificateContent = await value.certificate.text();
        } catch (error) {
          console.error('Failed to read certificate file:', error);
        }
      }
      console.log(certificateContent); // todo: delete

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
          cert: certificateContent,
        },
      };

      await createStream(requestData);
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
            {(field) => <field.FormUploadField title='Upload certificate file' />}
          </form.AppField>
        </form.AppForm>
      </form>
      <ModalControls
        submitProps={{
          text: m.controls_submit(),
          testId: 'add-destination-submit',
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
