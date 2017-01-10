setTimeout(function () {
  $('.alert-danger').hide(2000, function() {
    $(this).remove();
  });
}, 5000);

setInterval(function () {
  if ($('#auto-refresh').is(':checked'))
    location.reload()
}, 30000);

function pvpcompare() {
  const p1 = $("input:radio[name=player1]:checked").val();
  const p2 = $("input:radio[name=player2]:checked").val();

  window.location = "/analysis/pvp/" + p1 + "/" + p2;
}