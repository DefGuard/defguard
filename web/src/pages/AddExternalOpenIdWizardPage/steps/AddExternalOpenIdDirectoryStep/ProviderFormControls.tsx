import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { useFormContext } from '../../../../shared/form';

export const ProviderFormControls = ({ onBack }: { onBack: () => void }) => {
  const form = useFormContext();
  return (
    <form.Subscribe selector={(s) => s.isSubmitting}>
      {(isSubmitting) => (
        <Controls>
          <Button variant="outlined" text={m.controls_back()} onClick={onBack} />
          <div className="right">
            <Button
              text={m.controls_continue()}
              loading={isSubmitting}
              type="submit"
              onClick={() => {
                form.handleSubmit();
              }}
            />
          </div>
        </Controls>
      )}
    </form.Subscribe>
  );
};
