import './demo.scss';

import { Story } from '@ladle/react';

import SvgIconCheckmarkWhite from '../../../svg/IconCheckmarkWhite';
import SvgIconCheckmarkWhiteBig from '../../../svg/IconCheckmarkWhiteBig';
import { Button } from '../Button';
import { ButtonSize, ButtonStyleVariant } from '../types';

export const ButtonDemoStory: Story = () => {
  return (
    <>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          text="Text"
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Text"
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.CONFIRM}
          text="Text"
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          text="Text"
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Text"
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM}
          text="Text"
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.CONFIRM}
          text="Text"
          disabled
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Text"
          disabled
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM}
          text="Text"
          disabled
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.LINK}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.CONFIRM}
          text="Text"
          loading
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          text="Text"
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM}
          text="Text"
          loading
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
          rightIcon={<SvgIconCheckmarkWhiteBig />}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
          rightIcon={<SvgIconCheckmarkWhiteBig />}
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
          rightIcon={<SvgIconCheckmarkWhiteBig />}
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
          rightIcon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
          rightIcon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
          rightIcon={<SvgIconCheckmarkWhite />}
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
          rightIcon={<SvgIconCheckmarkWhiteBig />}
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
          rightIcon={<SvgIconCheckmarkWhiteBig />}
          loading
        />
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          icon={<SvgIconCheckmarkWhiteBig />}
          rightIcon={<SvgIconCheckmarkWhiteBig />}
          loading
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
          rightIcon={<SvgIconCheckmarkWhite />}
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
          rightIcon={<SvgIconCheckmarkWhite />}
          loading
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text="Text"
          icon={<SvgIconCheckmarkWhite />}
          rightIcon={<SvgIconCheckmarkWhite />}
          loading
        />
      </div>
      <div className="demo-buttons">
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.DELETE}
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.LINK}
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<SvgIconCheckmarkWhite />}
        />
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.CONFIRM}
          icon={<SvgIconCheckmarkWhite />}
        />
      </div>
    </>
  );
};

ButtonDemoStory.storyName = 'Demo';
