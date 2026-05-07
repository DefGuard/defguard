import { range } from 'radashi';
import { useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import { Card } from '../../../../../shared/components/Card/Card';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { DrawerModal } from '../../../../../shared/defguard-ui/components/DrawerModal/DrawerModal';

const testRange = Array.from(range(10)).map((idx) => (
  <p key={idx}>{m.test_placeholder_extreme()}</p>
));

export const PlaygroundTestDrawer = () => {
  const [isOpen, setIsOpen] = useState(false);
  return (
    <>
      <DrawerModal title="Test drawer" isOpen={isOpen} onClose={() => setIsOpen(false)}>
        {testRange}
        <Controls>
          <Button variant="outlined" text="smth" />
          <div className="right">
            <Button
              variant="secondary"
              text="Close"
              onClick={() => {
                setIsOpen(false);
              }}
            />
          </div>
        </Controls>
      </DrawerModal>
      <Card>
        <Button
          variant="primary"
          size="big"
          text="Test drawer"
          onClick={() => {
            setIsOpen(true);
          }}
        />
      </Card>
    </>
  );
};
