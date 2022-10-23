import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconNavLocations = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={24} height={24} {...props}>
    <defs>
      <style>{'\n      .icon-nav-locations_svg__b{fill:#899ca8}\n    '}</style>
    </defs>
    <path
      className="icon-nav-locations_svg__b"
      d="M19.134 2.946A9.939 9.939 0 0 0 12.044 0h-.01a10.08 10.08 0 0 0-10.03 10.047c.141 3.813 3.81 8.656 6.667 12.376a4.406 4.406 0 0 0 6.837-.106c2.119-2.634 6.7-8.968 6.557-12.27a10 10 0 0 0-2.927-7.1Zm-5.1 18.213a2.541 2.541 0 0 1-3.976 0c-3.874-4.932-6.18-9.04-6.18-11.112a8.173 8.173 0 0 1 8.156-8.171h.012a8.167 8.167 0 0 1 8.144 8.172c-.002 2.096-2.246 6.145-6.156 11.111Zm-1.948-7.143a3.984 3.984 0 1 1 3.984-3.984 3.989 3.989 0 0 1-3.985 3.984Zm0-6.093a2.109 2.109 0 1 0 2.109 2.109 2.112 2.112 0 0 0-2.11-2.109Z"
    />
  </svg>
);

export default SvgIconNavLocations;
