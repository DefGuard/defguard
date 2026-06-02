. /opt/bitnami/scripts/libopenldap.sh

ldap_start_bg

echo "Enabling memberof overlay for ${LDAP_ROOT}"

cat <<EOF | ldapmodify -Y EXTERNAL -H "ldapi:///"
dn: cn=module{0},cn=config
changetype: modify
add: olcModuleLoad
olcModuleLoad: /opt/bitnami/openldap/lib/openldap/memberof.so
EOF

cat <<EOF | ldapadd -Y EXTERNAL -H "ldapi:///"
dn: olcOverlay=memberof,olcDatabase={2}mdb,cn=config
objectClass: olcOverlayConfig
objectClass: olcMemberOfConfig
olcOverlay: memberof
olcMemberOfDangling: ignore
olcMemberOfRefInt: TRUE
olcMemberOfGroupOC: groupOfUniqueNames
olcMemberOfMemberAD: uniqueMember
EOF

ldap_stop
