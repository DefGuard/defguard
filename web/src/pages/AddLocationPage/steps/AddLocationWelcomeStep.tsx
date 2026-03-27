import { Controls } from '../../../shared/components/Controls/Controls';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationWelcomeStep = () => {
  return (
    <>
      <Divider spacing={ThemeSpacing.Xl2} />
      <Controls>
        <Button
          variant="primary"
          text={`Create new location`}
          onClick={() => {
            useAddLocationStore.setState({
              isWelcome: false,
            });
          }}
        />
      </Controls>
    </>
  );
};
