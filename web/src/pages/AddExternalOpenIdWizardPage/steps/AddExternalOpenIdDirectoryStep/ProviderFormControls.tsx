import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { useFormContext } from '../../../../shared/form';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';

export const ProviderFormControls = ({
  onBack,
  onNext,
  loading,
}: {
  onBack: () => void;
  onNext: () => void;
  loading?: boolean;
}) => {
  const form = useFormContext();
  const enabled = useAddExternalOpenIdStore(
    (s) => s.providerState.directory_sync_enabled,
  );

  return (
    <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
      {({ isSubmitting }) => (
        <Controls>
          <Button variant="outlined" text={m.controls_back()} onClick={onBack} />
          <div className="right">
            <Button
              text={m.controls_continue()}
              loading={isSubmitting || loading}
              type="submit"
              onClick={() => {
                if (enabled) {
                  form.handleSubmit();
                } else {
                  onNext();
                }
              }}
            />
          </div>
        </Controls>
      )}
    </form.Subscribe>
  );
};
