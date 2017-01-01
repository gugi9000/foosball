setTimeout(function () {
  $('.alert').hide(2000, function() {
    $(this).remove();
  });
}, 5000);

setInterval(function () {
  if ($('#auto-refresh').is(':checked'))
    location.reload()
}, 30000);
