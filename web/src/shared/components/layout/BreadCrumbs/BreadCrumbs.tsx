import './style.scss';

import { motion, Variants } from 'framer-motion';
import React, { useEffect, useState } from 'react';
import { NavLink, useLocation } from 'react-router-dom';

import { ColorsRGB } from '../../../constants';
import { titleCase } from '../../../utils/titleCase';
import SvgIconArrowGraySmall from '../../svg/IconArrowGraySmall';

interface BreadCrumb {
  path: string;
  label: string;
  active: boolean;
}

/**
 * Listen on `pathname` form `location` and displays small `nav` on the top of the container.
 *
 * Intended to use inside of `PageContainer` component.
 */
const BreadCrumbs: React.FC = () => {
  const [breadCrumbs, setBreadCrumbs] = useState<BreadCrumb[]>([]);

  const { pathname } = useLocation();

  useEffect(() => {
    const fullPath = pathname;
    const links = fullPath.split('/');
    const part: Array<BreadCrumb | undefined> = links.map((part, index) => {
      const path = links.slice(0, index + 1).join('/');
      const label = titleCase(part);
      if (path && path.length && label && label.length) {
        return {
          label: label,
          path: path,
          active: path === pathname,
        } as BreadCrumb;
      }
    });
    const res: BreadCrumb[] = part.filter(
      (val) => typeof val !== 'undefined'
    ) as BreadCrumb[];
    setBreadCrumbs([
      {
        path: '/',
        label: 'DefGuard',
        active: pathname === '/',
      },
      ...res,
    ]);
  }, [pathname]);

  return (
    <nav className="breadcrumbs">
      <motion.ul initial="hidden" animate="show" variants={navListVariants}>
        {breadCrumbs.map((item, index) => (
          <React.Fragment key={item.path}>
            <li key={item.label}>
              <NavLink to={item.path} replace={true}>
                <motion.span
                  key={item.label + item.active ? 'active' : ''}
                  whileHover="active"
                  whileTap="active"
                  animate={item.active ? 'active' : 'idle'}
                  variants={{
                    idle: {
                      color: ColorsRGB.GrayLight,
                    },
                    active: {
                      color: ColorsRGB.Primary,
                    },
                  }}
                >
                  {item.label}
                </motion.span>
              </NavLink>
              {index !== breadCrumbs.length - 1 ? (
                <SvgIconArrowGraySmall />
              ) : null}
            </li>
          </React.Fragment>
        ))}
      </motion.ul>
    </nav>
  );
};

export default BreadCrumbs;

const navListVariants: Variants = {
  hidden: {
    x: -20,
    opacity: 0,
  },
  show: {
    x: 0,
    opacity: 1,
  },
};
