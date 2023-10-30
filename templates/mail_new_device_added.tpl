{% extends "mail_base" %}
{% block content %}
<div style="background-color:#f9f9f9;">
  <div style="background:#f9f9f9;background-color:#f9f9f9;margin:0px auto;max-width:600px;">
    <table align="center" border="0" cellpadding="0" cellspacing="0" role="presentation"
      style="background:#f9f9f9;background-color:#f9f9f9;width:100%;">
      <tbody>
        <tr>
          <td style="direction:ltr;font-size:0px;padding:20px 0;text-align:center;">
            <div class="mj-column-per-100 mj-outlook-group-fix"
              style="font-size:0px;text-align:left;direction:ltr;display:inline-block;vertical-align:top;width:100%;">
              <table border="0" cellpadding="0" cellspacing="0" role="presentation" style="vertical-align:top;"
                width="100%">
                <tbody>
                  <tr>
                    <td align="center" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                      <div style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">
                        <p>A new device has been added to your account:</p>
                      </div>
                    </td>
                  </tr>
                  <tr>
                    <td align="center" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                      <div style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#899CA8;">
                        <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">
                          Name: {{ device_name }}</p>
                        <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">
                          Date: {{ date }}</p>
                        <!-- <a style="color:#899CA8;" target="_blank">Date: {{ date }}</a> -->
                        <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">IP
                          addresses:</p>
                        {% for ip_address in ip_addresses %}
                        <p style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">{{
                          ip_address }}</p>
                        {% endfor %}
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</div>
{% endblock content %}