import type { SVGProps } from 'react';
const SvgIconHamburgerClose = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={30}
    height={30}
    viewBox="0 0 30 30"
    {...props}
  >
    <defs>
      <clipPath id="icon-hamburger-close_svg__a">
        <path d="M-4-4h30v30H-4z" className="icon-hamburger-close_svg__a" />
      </clipPath>
      <clipPath id="icon-hamburger-close_svg__b">
        <path d="M0 0h22v22H0z" className="icon-hamburger-close_svg__a" />
      </clipPath>
      <style>
        {
          '.icon-hamburger-close_svg__a,.icon-hamburger-close_svg__c{fill:#899ca8}.icon-hamburger-close_svg__a{opacity:0}'
        }
      </style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-hamburger-close_svg__a)',
      }}
    >
      <g
        style={{
          clipPath: 'url(#icon-hamburger-close_svg__b)',
        }}
        transform="rotate(90 11 15)"
      >
        <rect
          width={19.998}
          height={2}
          className="icon-hamburger-close_svg__c"
          rx={1}
          transform="rotate(45 -1.57 7.208)"
        />
        <rect
          width={19.997}
          height={2}
          className="icon-hamburger-close_svg__c"
          rx={1}
          transform="rotate(135 8.429 6.208)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconHamburgerClose;
