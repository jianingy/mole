#!/usr/bin/env phantomjs
var webpage = require('webpage');

function init_page() {
    var page = webpage.create();
    page.onConsoleMessage = function(msg) {
        console.log(msg);
    };
    return page;
}

function make_proxyru(page_id) {
    return  function () {
        var page = init_page();
        console.log("[system] accessing proxy.com.ru - " + page_id);
        page.open("http://proxy.com.ru/list_" + page_id + ".html", function (status) {
            if (status === 'success') {
                page.evaluate(function () {
                    var rows = document.querySelectorAll("table")[7].querySelectorAll("tr");
                    for (var i = 1; i < rows.length; i++) {
                        var cells = rows[i].querySelectorAll("td");
                        if (cells.length < 3) {
                            continue;
                        }
                        console.info("[server] [proxyru] " + cells[1].innerText + ":" + cells[2].innerText);
                    }
                    return;
                });
            }
            next();
        });
    }
}

function make_kuai(page_id) {
    return  function () {
        var page = init_page();
        console.log("[system] accessing www.kuaidaili.com - " + page_id);
        page.open("http://www.kuaidaili.com/proxylist/" + page_id + "/", function (status) {
            if (status === 'success') {
                page.evaluate(function () {
                    var rows = document.querySelectorAll("div#index_free_list table tr");

                    for (var i = 1; i < rows.length; i++) {
                        var cells = rows[i].querySelectorAll("td");
                        if (cells.length < 2) {
                            continue;
                        }
                        console.info("[server] [kuai] " + cells[0].innerText + ":" + cells[1].innerText);
                    }
                    return;
                });
            }
            next();
        });
    }
}

function make_xici(type, page_id) {
    return  function () {
        var page = init_page();
        console.log("[system] accessing www.xicidaili.com - " + page_id);
        page.open("http://www.xicidaili.com/" + type + "/" + page_id, function (status) {
            if (status === 'success') {
                page.evaluate(function () {
                    var rows = document.querySelectorAll("table#ip_list tr");

                    for (var i = 1; i < rows.length; i++) {
                        var cells = rows[i].querySelectorAll("td");
                        if (cells.length < 3) {
                            continue;
                        }
                        console.info("[server] [xici] " + cells[1].innerText + ":" + cells[2].innerText);
                    }
                    return;
                });
            }
            next();
        });
    }
}

var sites = new Array(

    function () {
        var page = init_page();
        console.log("accessing www.proxy360.cn");
        page.open("http://www.proxy360.cn", function (status) {
            if (status === 'success') {
                page.evaluate(function () {
                    var rows = document.querySelectorAll("div.proxylistitem");
                    for (var i = 0; i < rows.length; i++) {
                        var cells = rows[i].querySelectorAll("span");
                        if (cells.length < 2) {
                            continue;
                        }
                        console.info("[server] [proxy360] " + cells[0].innerText + ":" + cells[1].innerText);
                    }
                    return;
                });
            }
            next();
        });
    }
);

for (i = 1; i < 3; i++) {
    sites.push(make_proxyru(i));
}

for (i = 1; i < 11; i++) {
    sites.push(make_kuai(i));
}

for (i = 1; i < 20; i++) {
    sites.push(make_xici('nn', i));
}

var next = function( ) {
    var site = sites.pop();
    if (site) {
        console.log("[system] starting in .5 seconds");
        setTimeout(site, 500);
    } else {
        phantom.exit();
    }
};

next();
