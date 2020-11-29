import {renderTemplate as tmpl} from "@utils/template";
import {appendId, appendValueLineId, getChildId, setValueId, toggleClasses, appendTextLineId, toggleClassesId, setTextId} from "@utils/dom";
import {modulePage, ModulePageKind} from "@components/module";
import {mockThemes} from "./common/mock-data";
import sidebarTmpl from "@templates/module/poster/edit/sidebar/sidebar.html";
import headerTmpl from "@templates/module/poster/edit/header.html";
import footerTmpl from "@templates/module/poster/edit/footer.html";
import mainTmpl from "@templates/module/poster/edit/main.html";
import layoutSidebar from "@templates/module/poster/edit/sidebar/layout.html";
import layoutSidebarItem from "@templates/module/poster/edit/sidebar/layout-item.html";


export default {
  title: 'Modules/Poster/Edit',
}


export const Layout = () => {
    const sidebar = makeSidebar(tmpl(layoutSidebar));
    mockThemes.forEach(({id, label, thumbnail}) => {
        const item = tmpl(layoutSidebarItem, {
            id, label, thumbnail
        });
        appendId(sidebar, "items", item);
    });
    
    return posterPage({sidebar});

}

function makeSidebar(child:Element) {
    const sidebar = tmpl(sidebarTmpl);
    sidebar.append(child);

    return sidebar
}

function posterPage({sidebar}:{sidebar: Element}) {
    const header = tmpl(headerTmpl, {
        title: "Create a Cover Page",
        subtitle: "Introduce your topic<br/>Use the blue panel for selecting layouts, themes, and adding content"
    });
    const footer = tmpl(footerTmpl);
    const main = tmpl(mainTmpl);

    return modulePage({
        kind: ModulePageKind.EditResize,
        sidebar,
        header,
        main,
        footer,
    })
}