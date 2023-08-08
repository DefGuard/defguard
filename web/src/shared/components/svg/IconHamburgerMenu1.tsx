import type { SVGProps } from 'react';
const SvgIconHamburgerMenu1 = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={30}
    height={30}
    viewBox="0 0 30 30"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h30v30H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
          transform="translate(-4 -4)"
        />
      </clipPath>
      <style>{'.c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="translate(-308 -223)"
    >
      <rect width={20} height={2} className="c" rx={1} transform="translate(313 231)" />
      <rect width={14} height={2} className="c" rx={1} transform="translate(313 237)" />
      <rect width={14} height={2} className="c" rx={1} transform="translate(313 243)" />
    </g>
  </svg>
);
export default SvgIconHamburgerMenu1;
