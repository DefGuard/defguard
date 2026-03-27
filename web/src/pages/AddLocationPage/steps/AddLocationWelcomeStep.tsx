import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationWelcomeStep = () => {
  return (
    <>
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <Button
          variant="primary"
          text={m.add_location_page_title()}
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
