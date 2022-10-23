import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconNavProfile = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={24} height={24} {...props}>
    <defs>
      <style>{'\n      .icon-nav-profile_svg__b{fill:#899ca8}\n    '}</style>
    </defs>
    <path
      className="icon-nav-profile_svg__b"
      d="M20.531 24a.937.937 0 0 1-.937-.937 6.664 6.664 0 0 0-6.656-6.656h-1.407a6.664 6.664 0 0 0-6.656 6.656.938.938 0 0 1-1.875 0 8.541 8.541 0 0 1 8.531-8.532h1.406a8.541 8.541 0 0 1 8.531 8.531.937.937 0 0 1-.937.938Zm-8.39-11.344a6.328 6.328 0 1 1 6.328-6.328 6.335 6.335 0 0 1-6.328 6.328Zm0-10.781a4.453 4.453 0 1 0 4.453 4.453 4.458 4.458 0 0 0-4.453-4.453Z"
    />
  </svg>
);

export default SvgIconNavProfile;
