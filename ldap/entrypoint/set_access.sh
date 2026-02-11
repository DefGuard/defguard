. /opt/bitnami/scripts/libopenldap.sh

ldap_start_bg

echo "Setting custom access permissions for ${LDAP_ROOT}"

cat <<EOF | ldapmodify -Y EXTERNAL -H "ldapi:///"
dn: olcDatabase={-1}frontend,cn=config
changetype: modify
replace: olcAccess
olcAccess: to attrs=userPassword,shadowLastChange
  by self write
  by group/groupOfUniqueNames/uniqueMember.exact="cn=admin,ou=groups,${LDAP_ROOT}" write
  by anonymous auth
olcAccess: to *
  by self write
  by group/groupOfUniqueNames/uniqueMember.exact="cn=admin,ou=groups,${LDAP_ROOT}" write
  by * read
EOF


ldap_stop
