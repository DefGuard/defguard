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
      <clipPath id="a">
        <path d="M-4-4h30v30H-4z" className="a" />
      </clipPath>
      <clipPath id="b">
        <path d="M0 0h22v22H0z" className="a" />
      </clipPath>
      <style>{'.a,.c{fill:#899ca8}.a{opacity:0}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
    >
      <g
        style={{
          clipPath: 'url(#b)',
        }}
        transform="rotate(90 11 15)"
      >
        <rect
          width={19.998}
          height={2}
          className="c"
          rx={1}
          transform="rotate(45 -1.57 7.208)"
        />
        <rect
          width={19.997}
          height={2}
          className="c"
          rx={1}
          transform="rotate(135 8.429 6.208)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconHamburgerClose;
